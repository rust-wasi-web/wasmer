use super::*;
use crate::{journal::SnapshotTrigger, syscalls::*};

/// ### `sock_listen()`
/// Listen for connections on a socket
///
/// Polling the socket handle will wait until a connection
/// attempt is made
///
/// Note: This is similar to `listen`
///
/// ## Parameters
///
/// * `fd` - File descriptor of the socket to be bind
/// * `backlog` - Maximum size of the queue for pending connections
#[instrument(level = "trace", skip_all, fields(%sock, %backlog), ret)]
pub fn sock_listen<M: MemorySize>(
    mut ctx: FunctionEnvMut<'_, WasiEnv>,
    sock: WasiFd,
    backlog: M::Offset,
) -> Result<Errno, WasiError> {
    let env = ctx.data();
    let backlog: usize = wasi_try_ok!(backlog.try_into().map_err(|_| Errno::Inval));

    wasi_try_ok!(sock_listen_internal(&mut ctx, sock, backlog)?);

    Ok(Errno::Success)
}

pub(crate) fn sock_listen_internal(
    ctx: &mut FunctionEnvMut<'_, WasiEnv>,
    sock: WasiFd,
    backlog: usize,
) -> Result<Result<(), Errno>, WasiError> {
    let env = ctx.data();
    let net = env.net().clone();
    let tasks = ctx.data().tasks().clone();
    wasi_try_ok_ok!(__sock_upgrade(
        ctx,
        sock,
        Rights::SOCK_LISTEN,
        |socket, _| async move { socket.listen(tasks.deref(), net.deref(), backlog).await }
    ));

    Ok(Ok(()))
}
