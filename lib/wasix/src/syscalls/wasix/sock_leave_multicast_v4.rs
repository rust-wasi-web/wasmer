use super::*;
use crate::syscalls::*;

/// ### `sock_leave_multicast_v4()`
/// Leaves a particular multicast IPv4 group
///
/// ## Parameters
///
/// * `fd` - Socket descriptor
/// * `multiaddr` - Multicast group to leave
/// * `interface` - Interface that will left
#[instrument(level = "trace", skip_all, fields(%sock), ret)]
pub fn sock_leave_multicast_v4<M: MemorySize>(
    mut ctx: FunctionEnvMut<'_, WasiEnv>,
    sock: WasiFd,
    multiaddr: WasmPtr<__wasi_addr_ip4_t, M>,
    iface: WasmPtr<__wasi_addr_ip4_t, M>,
) -> Result<Errno, WasiError> {
    let env = ctx.data();
    let memory = unsafe { env.memory_view(&ctx) };
    let multiaddr = wasi_try_ok!(crate::net::read_ip_v4(&memory, multiaddr));
    let iface = wasi_try_ok!(crate::net::read_ip_v4(&memory, iface));

    wasi_try_ok!(sock_leave_multicast_v4_internal(
        &mut ctx, sock, multiaddr, iface
    )?);

    Ok(Errno::Success)
}

pub(crate) fn sock_leave_multicast_v4_internal(
    ctx: &mut FunctionEnvMut<'_, WasiEnv>,
    sock: WasiFd,
    multiaddr: Ipv4Addr,
    iface: Ipv4Addr,
) -> Result<Result<(), Errno>, WasiError> {
    wasi_try_ok_ok!(__sock_actor_mut(ctx, sock, Rights::empty(), |socket, _| {
        socket.leave_multicast_v4(multiaddr, iface)
    }));
    Ok(Ok(()))
}
