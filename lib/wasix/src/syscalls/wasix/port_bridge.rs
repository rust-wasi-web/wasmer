use super::*;
use crate::syscalls::*;

/// ### `port_bridge()`
/// Securely connects to a particular remote network
///
/// ## Parameters
///
/// * `network` - Fully qualified identifier for the network
/// * `token` - Access token used to authenticate with the network
/// * `security` - Level of encryption to encapsulate the network connection with
#[instrument(level = "trace", skip_all, fields(network = field::Empty, ?security), ret)]
pub fn port_bridge<M: MemorySize>(
    mut ctx: FunctionEnvMut<'_, WasiEnv>,
    network: WasmPtr<u8, M>,
    network_len: M::Offset,
    token: WasmPtr<u8, M>,
    token_len: M::Offset,
    security: Streamsecurity,
) -> Result<Errno, WasiError> {
    let env = ctx.data();
    let memory = unsafe { env.memory_view(&ctx) };

    let network = get_input_str_ok!(&memory, network, network_len);
    Span::current().record("network", network.as_str());

    let token = get_input_str_ok!(&memory, token, token_len);
    let security = match security {
        Streamsecurity::Unencrypted => StreamSecurity::Unencrypted,
        Streamsecurity::AnyEncryption => StreamSecurity::AnyEncyption,
        Streamsecurity::ClassicEncryption => StreamSecurity::ClassicEncryption,
        Streamsecurity::DoubleEncryption => StreamSecurity::DoubleEncryption,
        _ => return Ok(Errno::Inval),
    };

    wasi_try_ok!(port_bridge_internal(
        &mut ctx,
        network.as_str(),
        token.as_str(),
        security
    )?);

    Ok(Errno::Success)
}

pub(crate) fn port_bridge_internal(
    ctx: &mut FunctionEnvMut<'_, WasiEnv>,
    network: &str,
    token: &str,
    security: StreamSecurity,
) -> Result<Result<(), Errno>, WasiError> {
    let env = ctx.data();

    let net = env.net().clone();
    wasi_try_ok_ok!(block_on_with_signals(ctx, None, async move {
        net.bridge(network, token, security)
            .await
            .map_err(net_error_into_wasi_err)
    })?);
    Ok(Ok(()))
}
