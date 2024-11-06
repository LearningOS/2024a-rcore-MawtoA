//! File and filesystem-related syscalls

use crate::mm::translated_byte_buffer;
use crate::syscall::SYSCALL_WRITE;
use crate::task::{add_syscall_count, current_user_token};

const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel: sys_write");
    // 此时更新调用次数
    add_syscall_count(SYSCALL_WRITE);
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
