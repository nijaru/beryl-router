use aya_ebpf::{
    macros::{classifier, map},
    maps::HashMap,
    programs::TcContext,
};
use aya_log_ebpf::info;
use beryl_common::PacketAction;
use core::mem;
use network_types::{
    eth::{EthHdr, EtherType},
    ip::Ipv4Hdr,
};

/// Egress blocklist: destination IP -> action (0 = pass, 1 = drop)
#[map]
static EGRESS_BLOCK: HashMap<u32, u32> = HashMap::with_max_entries(4096, 0);

#[classifier]
pub fn tc_egress(ctx: TcContext) -> i32 {
    match try_tc_egress(ctx) {
        Ok(ret) => ret,
        Err(_) => 0, // TC_ACT_OK
    }
}

#[inline(always)]
fn ptr_at<T>(ctx: &TcContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

fn try_tc_egress(ctx: TcContext) -> Result<i32, ()> {
    // Parse Ethernet header
    let eth_hdr: *const EthHdr = ptr_at(&ctx, 0)?;
    let eth_type = unsafe { (*eth_hdr).ether_type };

    // Only process IPv4
    if eth_type != EtherType::Ipv4 {
        return Ok(0); // TC_ACT_OK
    }

    // Parse IPv4 header
    let ipv4_hdr: *const Ipv4Hdr = ptr_at(&ctx, EthHdr::LEN)?;
    let dst_ip = u32::from_be(unsafe { (*ipv4_hdr).dst_addr });

    // Check egress blocklist
    if let Some(&action) = unsafe { EGRESS_BLOCK.get(&dst_ip) } {
        if action == PacketAction::Drop as u32 {
            info!(&ctx, "TC DROP: blocked egress IP {:i}", dst_ip);
            return Ok(2); // TC_ACT_SHOT (Drop)
        }
    }

    Ok(0) // TC_ACT_OK
}
