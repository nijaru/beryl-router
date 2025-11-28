use anyhow::Result;
use beryl_dhcp::database::LeaseDatabase;
use hickory_resolver::{
    TokioAsyncResolver,
    config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts},
};
use hickory_server::{
    authority::MessageResponseBuilder,
    proto::op::{Header, ResponseCode},
    proto::rr::{Name, RData, Record, RecordType, rdata::A},
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Forwarder {
    resolver: Arc<TokioAsyncResolver>,
    dhcp_db: Arc<RwLock<LeaseDatabase>>,
    local_domain: Option<String>,
}

impl Forwarder {
    pub fn new(
        upstreams: Vec<SocketAddr>,
        dhcp_db: Arc<RwLock<LeaseDatabase>>,
        local_domain: Option<String>,
    ) -> Result<Self> {
        let mut config = ResolverConfig::new();
        for addr in upstreams {
            config.add_name_server(NameServerConfig::new(addr, Protocol::Udp));
            config.add_name_server(NameServerConfig::new(addr, Protocol::Tcp));
        }

        if config.name_servers().is_empty() {
            config = ResolverConfig::cloudflare();
        }

        let opts = ResolverOpts::default();
        let resolver = TokioAsyncResolver::tokio(config, opts);

        Ok(Self {
            resolver: Arc::new(resolver),
            dhcp_db,
            local_domain,
        })
    }

    async fn resolve_local(&self, name: &Name) -> Option<Vec<Record>> {
        let name_str = name.to_string();
        // Strip trailing dot
        let name_str = name_str.trim_end_matches('.');

        // Check if it matches local domain
        let hostname = if let Some(domain) = &self.local_domain {
            if name_str.ends_with(domain) {
                name_str
                    .strip_suffix(domain)?
                    .strip_suffix('.')
                    .unwrap_or(name_str.strip_suffix(domain)?)
            } else {
                name_str
            }
        } else {
            name_str
        };

        let db = self.dhcp_db.read().await;
        if let Some(ip) = db.get_ip_by_hostname(hostname) {
            let mut record = Record::new();
            record
                .set_name(name.clone())
                .set_rr_type(RecordType::A)
                .set_ttl(60)
                .set_data(Some(RData::A(A::from(ip)))); // set_data takes Option<RData>
            return Some(vec![record]);
        }

        None
    }
}

#[async_trait::async_trait]
impl RequestHandler for Forwarder {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let query = request.query();
        // Convert LowerName to Name for our usage
        let name: Name = query.name().into();
        let query_type = query.query_type();

        // Build standard response header
        let mut header = Header::response_from_request(request.header());
        header.set_recursion_available(true);
        header.set_authoritative(false);

        let builder = MessageResponseBuilder::from_message_request(request);

        // 1. Try Local Resolution first (for A records)
        if query_type == RecordType::A {
            if let Some(records) = self.resolve_local(&name).await {
                let response = builder.build(header, records.iter(), &[], &[], &[]);
                return match response_handle.send_response(response).await {
                    Ok(info) => info,
                    Err(e) => {
                        tracing::error!("Failed to send local response: {}", e);
                        ResponseInfo::from(header)
                    }
                };
            }
        }

        // 2. Forward to upstream
        match self.resolver.lookup(name, query_type).await {
            Ok(lookup) => {
                let records = lookup.records();
                let response = builder.build(header, records.iter(), &[], &[], &[]);

                match response_handle.send_response(response).await {
                    Ok(info) => info,
                    Err(e) => {
                        tracing::error!("Failed to send response: {}", e);
                        ResponseInfo::from(header)
                    }
                }
            }
            Err(e) => {
                use hickory_resolver::error::ResolveErrorKind;
                let response_code = match e.kind() {
                    ResolveErrorKind::NoRecordsFound { .. } => ResponseCode::NXDomain,
                    ResolveErrorKind::Proto(_) => ResponseCode::ServFail,
                    ResolveErrorKind::Timeout => ResponseCode::ServFail,
                    _ => ResponseCode::ServFail,
                };

                let response = builder.error_msg(request.header(), response_code);
                match response_handle.send_response(response).await {
                    Ok(info) => info,
                    Err(e) => {
                        tracing::error!("Failed to send error response: {}", e);
                        ResponseInfo::from(header)
                    }
                }
            }
        }
    }
}
