use super::*;
use crate::syscalls::*;

/// ### `path_symlink()`
/// Create a symlink
/// Inputs:
/// - `const char *old_path`
///     Array of UTF-8 bytes representing the source path
/// - `u32 old_path_len`
///     The number of bytes to read from `old_path`
/// - `Fd fd`
///     The base directory from which the paths are understood
/// - `const char *new_path`
///     Array of UTF-8 bytes representing the target path
/// - `u32 new_path_len`
///     The number of bytes to read from `new_path`
#[instrument(level = "trace", skip_all, fields(%fd, old_path = field::Empty, new_path = field::Empty), ret)]
pub fn path_symlink<M: MemorySize>(
    mut ctx: FunctionEnvMut<'_, WasiEnv>,
    old_path: WasmPtr<u8, M>,
    old_path_len: M::Offset,
    fd: WasiFd,
    new_path: WasmPtr<u8, M>,
    new_path_len: M::Offset,
) -> Result<Errno, WasiError> {
    let env = ctx.data();
    let (memory, _state, _inodes) = unsafe { env.get_memory_and_wasi_state_and_inodes(&ctx, 0) };
    let mut old_path_str = get_input_str_ok!(&memory, old_path, old_path_len);
    Span::current().record("old_path", old_path_str.as_str());
    let mut new_path_str = get_input_str_ok!(&memory, new_path, new_path_len);
    Span::current().record("new_path", new_path_str.as_str());
    old_path_str = ctx.data().state.fs.relative_path_to_absolute(old_path_str);
    new_path_str = ctx.data().state.fs.relative_path_to_absolute(new_path_str);

    wasi_try_ok!(path_symlink_internal(
        &mut ctx,
        &old_path_str,
        fd,
        &new_path_str
    ));

    Ok(Errno::Success)
}

pub fn path_symlink_internal(
    ctx: &mut FunctionEnvMut<'_, WasiEnv>,
    old_path: &str,
    fd: WasiFd,
    new_path: &str,
) -> Result<(), Errno> {
    let env = ctx.data();
    let (_memory, state, inodes) = unsafe { env.get_memory_and_wasi_state_and_inodes(&ctx, 0) };

    let base_fd = state.fs.get_fd(fd)?;
    if !base_fd.rights.contains(Rights::PATH_SYMLINK) {
        return Err(Errno::Access);
    }

    // get the depth of the parent + 1 (UNDER INVESTIGATION HMMMMMMMM THINK FISH ^ THINK FISH)
    let old_path_path = std::path::Path::new(old_path);
    let (source_inode, _) = state
        .fs
        .get_parent_inode_at_path(inodes, fd, old_path_path, true)?;
    let depth = state.fs.path_depth_from_fd(fd, source_inode);

    // depth == -1 means folder is not relative. See issue #3233.
    let depth = match depth {
        Ok(depth) => depth as i32 - 1,
        Err(_) => -1,
    };

    let new_path_path = std::path::Path::new(new_path);
    let (target_parent_inode, entry_name) =
        state
            .fs
            .get_parent_inode_at_path(inodes, fd, new_path_path, true)?;

    // short circuit if anything is wrong, before we create an inode
    {
        let guard = target_parent_inode.read();
        match guard.deref() {
            Kind::Dir { entries, .. } => {
                if entries.contains_key(&entry_name) {
                    return Err(Errno::Exist);
                }
            }
            Kind::Root { .. } => return Err(Errno::Notcapable),
            Kind::Socket { .. }
            | Kind::Pipe { .. }
            | Kind::EventNotifications { .. }
            | Kind::Epoll { .. } => return Err(Errno::Inval),
            Kind::File { .. } | Kind::Symlink { .. } | Kind::Buffer { .. } => {
                unreachable!("get_parent_inode_at_path returned something other than a Dir or Root")
            }
        }
    }

    let source_path = std::path::Path::new(old_path);
    let mut relative_path = std::path::PathBuf::new();
    for _ in 0..depth {
        relative_path.push("..");
    }
    relative_path.push(source_path);

    let kind = Kind::Symlink {
        base_po_dir: fd,
        path_to_symlink: std::path::PathBuf::from(new_path),
        relative_path,
    };
    let new_inode =
        state
            .fs
            .create_inode_with_default_stat(inodes, kind, false, entry_name.clone().into());

    {
        let mut guard = target_parent_inode.write();
        if let Kind::Dir {
            ref mut entries, ..
        } = guard.deref_mut()
        {
            entries.insert(entry_name, new_inode);
        }
    }

    Ok(())
}
