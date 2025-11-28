#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, PerCpuArray},
    programs::XdpContext,
};
use aya_log_ebpf::info;
use beryl_common::{PacketAction, Stats};
mod tc_egress;
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpProto, Ipv4Hdr},
    tcp::TcpHdr,
    udp::UdpHdr,
};

/// Blocklist: IPv4 address -> action (0 = pass, 1 = drop)
#[map]
static BLOCKLIST: HashMap<u32, u32> = HashMap::with_max_entries(4096, 0);

/// Port blocklist: port number -> action (0 = pass, 1 = drop)
#[map]
static PORT_BLOCKLIST: HashMap<u16, u32> = HashMap::with_max_entries(1024, 0);

/// Per-CPU statistics
#[map]
static STATS: PerCpuArray<Stats> = PerCpuArray::with_max_entries(1, 0);

#[xdp]
pub fn xdp_firewall(ctx: XdpContext) -> u32 {
    match try_xdp_firewall(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

#[inline(always)]
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

fn try_xdp_firewall(ctx: XdpContext) -> Result<u32, ()> {
    // Update packet counter
    if let Some(stats) = unsafe { STATS.get_ptr_mut(0) } {
        unsafe { (*stats).packets_total += 1 };
    }

    // Parse Ethernet header
    let eth_hdr: *const EthHdr = ptr_at(&ctx, 0)?;
    let eth_type = unsafe { (*eth_hdr).ether_type };

    // Only process IPv4
    if eth_type != EtherType::Ipv4 {
        return Ok(xdp_action::XDP_PASS);
    }

    // Parse IPv4 header
    let ipv4_hdr: *const Ipv4Hdr = ptr_at(&ctx, EthHdr::LEN)?;
    let src_ip = u32::from_be(unsafe { (*ipv4_hdr).src_addr });
    let proto = unsafe { (*ipv4_hdr).proto };

    // Check IP blocklist
    if let Some(&action) = unsafe { BLOCKLIST.get(&src_ip) } {
        if action == PacketAction::Drop as u32 {
            if let Some(stats) = unsafe { STATS.get_ptr_mut(0) } {
                unsafe { (*stats).packets_dropped += 1 };
            }
            info!(&ctx, "DROP: blocked IP {:i}", src_ip);
            return Ok(xdp_action::XDP_DROP);
        }
    }

    // Check port blocklist for TCP/UDP
    let ip_hdr_len = ((unsafe { (*ipv4_hdr).ihl() }) as usize) * 4;
    let transport_offset = EthHdr::LEN + ip_hdr_len;

    let dst_port: u16 = match proto {
        IpProto::Tcp => {
            let tcp_hdr: *const TcpHdr = ptr_at(&ctx, transport_offset)?;
            u16::from_be(unsafe { (*tcp_hdr).dest })
        }
        IpProto::Udp => {
            let udp_hdr: *const UdpHdr = ptr_at(&ctx, transport_offset)?;
            u16::from_be(unsafe { (*udp_hdr).dest })
        }
        _ => 0,
    };

    if dst_port != 0 {
        if let Some(&action) = unsafe { PORT_BLOCKLIST.get(&dst_port) } {
            if action == PacketAction::Drop as u32 {
                if let Some(stats) = unsafe { STATS.get_ptr_mut(0) } {
                    unsafe { (*stats).packets_dropped += 1 };
                }
                info!(&ctx, "DROP: blocked port {}", dst_port);
                return Ok(xdp_action::XDP_DROP);
            }
        }
    }

    // Update passed counter
    if let Some(stats) = unsafe { STATS.get_ptr_mut(0) } {
        unsafe { (*stats).packets_passed += 1 };
    }

    Ok(xdp_action::XDP_PASS)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
