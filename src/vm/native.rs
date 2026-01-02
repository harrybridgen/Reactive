use super::{NativeFunction, VM};
use crate::grammar::Type;
use std::collections::HashSet;
#[cfg(unix)]
use std::collections::VecDeque;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{BOOL, HANDLE, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode, GetStdHandle,
    STD_INPUT_HANDLE, SetConsoleCtrlHandler, SetConsoleMode,
};

impl VM {
    pub(crate) fn install_native_fs(&mut self) {
        self.register_native("internal_file_read", native_read);
        self.register_native("internal_file_write", native_write);
        self.register_native("internal_file_exists", native_exists);
        self.register_native("internal_file_remove", native_remove);
    }

    pub(crate) fn install_native_buf(&mut self) {
        self.register_native("internal_buf_new", native_buf_new);
        self.register_native("internal_buf_push_char", native_buf_push_char);
        self.register_native("internal_buf_push_str", native_buf_push_str);
        self.register_native("internal_buf_to_string", native_buf_to_string);
        self.register_native("internal_buf_write_file", native_buf_write_file);
    }

    pub(crate) fn install_native_vec(&mut self) {
        self.register_native("internal_vec_new", native_vec_new);
        self.register_native("internal_vec_push", native_vec_push);
        self.register_native("internal_vec_pop", native_vec_pop);
    }

    pub(crate) fn install_native_input(&mut self) {
        self.register_native("internal_input_readline", native_input_readline);
        self.register_native("internal_input_init", native_input_init);
        self.register_native("internal_input_poll", native_input_poll);
        self.register_native("internal_input_shutdown", native_input_shutdown);
    }

    fn register_native(&mut self, name: &str, f: NativeFunction) {
        self.native_functions.insert(name.to_string(), f);
        self.global_env
            .insert(name.to_string(), Type::NativeFunction(name.to_string()));
    }

    fn value_to_string(&mut self, v: Type, what: &str) -> String {
        match self.force(v) {
            Type::ArrayRef(id) => {
                let elems = self.array_heap[id].clone();
                let mut out = String::with_capacity(elems.len());
                for elem in elems {
                    match self.force(elem) {
                        Type::Char(c) => match char::from_u32(c) {
                            Some(ch) => out.push(ch),
                            None => self
                                .runtime_error(&format!("{what} contains invalid char code {c}")),
                        },
                        other => self.runtime_error(&format!(
                            "{what} must be a string (array of chars), found {:?}",
                            other
                        )),
                    }
                }
                out
            }
            Type::VecRef(id) => {
                let elems = self.vec_heap[id].clone();
                let mut out = String::with_capacity(elems.len());
                for elem in elems {
                    match self.force(elem) {
                        Type::Char(c) => match char::from_u32(c) {
                            Some(ch) => out.push(ch),
                            None => self
                                .runtime_error(&format!("{what} contains invalid char code {c}")),
                        },
                        other => self.runtime_error(&format!(
                            "{what} must be a string (array of chars), found {:?}",
                            other
                        )),
                    }
                }
                out
            }
            other => self.runtime_error(&format!(
                "{what} must be a string (array of chars), found {:?}",
                other
            )),
        }
    }

    fn string_to_array(&mut self, s: &str) -> Type {
        let id = self.array_heap.len();
        let elems: Vec<Type> = s.chars().map(|ch| Type::Char(ch as u32)).collect();
        self.array_heap.push(elems);
        self.array_immutables.push(HashSet::new());
        Type::ArrayRef(id)
    }
}

fn native_read(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_file_read expects 1 argument, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_read path");
    let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        vm.runtime_error(&format!("internal_file_read failed for `{}`: {}", path, e))
    });
    vm.string_to_array(&contents)
}

fn native_write(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_file_write expects 2 arguments, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_write path");
    let contents = vm.value_to_string(args[1].clone(), "internal_file_write contents");

    std::fs::write(&path, contents.as_bytes()).unwrap_or_else(|e| {
        vm.runtime_error(&format!("internal_file_write failed for `{}`: {}", path, e))
    });

    let count = contents.chars().count();
    let count_i32 = i32::try_from(count)
        .unwrap_or_else(|_| vm.runtime_error("write contents too large for int"));
    Type::Integer(count_i32)
}

fn native_exists(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_file_exists expects 1 argument, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_exists path");
    let exists = Path::new(&path).exists();
    Type::Integer(if exists { 1 } else { 0 })
}

fn native_remove(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_file_remove expects 1 argument, got {}",
            args.len()
        ));
    }

    let path = vm.value_to_string(args[0].clone(), "internal_file_remove path");
    std::fs::remove_file(&path).unwrap_or_else(|e| {
        vm.runtime_error(&format!(
            "internal_file_remove failed for `{}`: {}",
            path, e
        ))
    });
    Type::Integer(1)
}

fn native_buf_new(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_buf_new expects 1 argument, got {}",
            args.len()
        ));
    }

    let cap = vm.as_usize_nonneg(args[0].clone(), "internal_buf_new capacity");
    let id = vm.buffer_heap.len();
    vm.buffer_heap.push(Vec::with_capacity(cap));
    Type::BufferRef(id)
}

fn native_buf_push_char(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_buf_push_char expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_push_char expects buffer, found {:?}",
            other
        )),
    };

    let ch = match vm.force(args[1].clone()) {
        Type::Char(c) => c,
        other => vm.runtime_error(&format!(
            "internal_buf_push_char expects char, found {:?}",
            other
        )),
    };

    vm.buffer_heap[id].push(ch);
    Type::BufferRef(id)
}

fn native_buf_push_str(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_buf_push_str expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_push_str expects buffer, found {:?}",
            other
        )),
    };

    let str_id = match vm.force(args[1].clone()) {
        Type::ArrayRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_push_str expects string, found {:?}",
            other
        )),
    };

    let elems = vm.array_heap[str_id].clone();
    for elem in elems {
        match vm.force(elem) {
            Type::Char(c) => vm.buffer_heap[id].push(c),
            other => vm.runtime_error(&format!(
                "internal_buf_push_str expects string of chars, found {:?}",
                other
            )),
        }
    }

    Type::BufferRef(id)
}

fn native_buf_to_string(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_buf_to_string expects 1 argument, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_to_string expects buffer, found {:?}",
            other
        )),
    };

    let elems: Vec<Type> = vm.buffer_heap[id].iter().map(|c| Type::Char(*c)).collect();
    let arr_id = vm.array_heap.len();
    vm.array_heap.push(elems);
    vm.array_immutables.push(HashSet::new());
    Type::ArrayRef(arr_id)
}

fn native_buf_write_file(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_buf_write_file expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::BufferRef(id) => id,
        other => vm.runtime_error(&format!(
            "internal_buf_write_file expects buffer, found {:?}",
            other
        )),
    };
    let path = vm.value_to_string(args[1].clone(), "internal_buf_write_file path");

    let mut file = std::fs::File::create(&path).unwrap_or_else(|e| {
        vm.runtime_error(&format!(
            "internal_buf_write_file failed for `{}`: {}",
            path, e
        ))
    });

    let mut count = 0usize;
    for c in vm.buffer_heap[id].iter().copied() {
        let ch = char::from_u32(c)
            .unwrap_or_else(|| vm.runtime_error(&format!("invalid char code {c} in buffer")));
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        file.write_all(encoded.as_bytes()).unwrap_or_else(|e| {
            vm.runtime_error(&format!(
                "internal_buf_write_file failed for `{}`: {}",
                path, e
            ))
        });
        count += 1;
    }

    let count_i32 =
        i32::try_from(count).unwrap_or_else(|_| vm.runtime_error("buffer too large for int"));
    Type::Integer(count_i32)
}

fn native_vec_new(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_vec_new expects 1 argument, got {}",
            args.len()
        ));
    }

    let cap = vm.as_usize_nonneg(args[0].clone(), "internal_vec_new capacity");
    let id = vm.vec_heap.len();
    vm.vec_heap.push(Vec::with_capacity(cap));
    vm.vec_immutables.push(HashSet::new());
    Type::VecRef(id)
}

fn native_vec_push(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 2 {
        vm.runtime_error(&format!(
            "internal_vec_push expects 2 arguments, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::VecRef(id) => id,
        other => vm.runtime_error(&format!("internal_vec_push expects vec, found {:?}", other)),
    };

    let val = args[1].clone();
    vm.vec_heap[id].push(val);
    Type::VecRef(id)
}

fn native_vec_pop(vm: &mut VM, args: Vec<Type>) -> Type {
    if args.len() != 1 {
        vm.runtime_error(&format!(
            "internal_vec_pop expects 1 argument, got {}",
            args.len()
        ));
    }

    let id = match vm.force(args[0].clone()) {
        Type::VecRef(id) => id,
        other => vm.runtime_error(&format!("internal_vec_pop expects vec, found {:?}", other)),
    };

    let value = vm.vec_heap[id]
        .pop()
        .unwrap_or_else(|| vm.runtime_error("internal_vec_pop on empty vec"));
    let len = vm.vec_heap[id].len();
    vm.vec_immutables[id].remove(&len);
    value
}

const KEY_UP: i32 = 1000;
const KEY_DOWN: i32 = 1001;
const KEY_LEFT: i32 = 1002;
const KEY_RIGHT: i32 = 1003;

fn native_input_readline(vm: &mut VM, args: Vec<Type>) -> Type {
    if !args.is_empty() {
        vm.runtime_error(&format!(
            "internal_input_readline expects 0 arguments, got {}",
            args.len()
        ));
    }

    #[cfg(unix)]
    let restore = unix_suspend_raw_input();

    #[cfg(windows)]
    let restore = win_suspend_raw_input();

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .unwrap_or_else(|e| vm.runtime_error(&format!("internal_input_readline failed: {e}")));

    #[cfg(unix)]
    unix_restore_raw_input(restore);

    #[cfg(windows)]
    win_restore_raw_input(restore);

    while line.ends_with('\n') || line.ends_with('\r') {
        line.pop();
    }
    vm.string_to_array(&line)
}

fn native_input_init(vm: &mut VM, args: Vec<Type>) -> Type {
    if !args.is_empty() {
        vm.runtime_error(&format!(
            "internal_input_init expects 0 arguments, got {}",
            args.len()
        ));
    }

    #[cfg(unix)]
    unix_input_init(vm);

    #[cfg(windows)]
    win_input_init(vm);

    Type::Integer(0)
}

fn native_input_poll(vm: &mut VM, args: Vec<Type>) -> Type {
    if !args.is_empty() {
        vm.runtime_error(&format!(
            "internal_input_poll expects 0 arguments, got {}",
            args.len()
        ));
    }

    #[cfg(unix)]
    return Type::Integer(unix_input_poll(vm));

    #[cfg(windows)]
    return Type::Integer(win_input_poll(vm));

    #[cfg(not(any(unix, windows)))]
    {
        vm.runtime_error("internal_input_poll is not supported on this platform");
    }
}

fn native_input_shutdown(vm: &mut VM, args: Vec<Type>) -> Type {
    if !args.is_empty() {
        vm.runtime_error(&format!(
            "internal_input_shutdown expects 0 arguments, got {}",
            args.len()
        ));
    }

    #[cfg(unix)]
    unix_input_shutdown(vm);

    #[cfg(windows)]
    win_input_shutdown(vm);

    Type::Integer(0)
}

#[cfg(unix)]
struct UnixInputState {
    fd: i32,
    orig_termios: libc::termios,
    raw_termios: libc::termios,
    orig_flags: i32,
    raw_flags: i32,
    pending: VecDeque<u8>,
}

#[cfg(unix)]
fn unix_state() -> &'static Mutex<Option<UnixInputState>> {
    static STATE: OnceLock<Mutex<Option<UnixInputState>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

#[cfg(unix)]
fn unix_register_atexit() {
    static REGISTER: OnceLock<()> = OnceLock::new();
    if REGISTER.set(()).is_ok() {
        unsafe {
            libc::atexit(unix_atexit);
        }
    }
}

#[cfg(unix)]
extern "C" fn unix_atexit() {
    unix_restore_state();
}

#[cfg(unix)]
fn unix_restore_state() {
    if let Some(state) = unix_state().lock().ok().and_then(|mut guard| guard.take()) {
        unsafe {
            libc::tcsetattr(state.fd, libc::TCSANOW, &state.orig_termios);
            libc::fcntl(state.fd, libc::F_SETFL, state.orig_flags);
        }
    }
}

#[cfg(unix)]
fn unix_input_init(vm: &mut VM) {
    let mut guard = unix_state()
        .lock()
        .unwrap_or_else(|_| vm.runtime_error("input state lock poisoned"));
    if guard.is_some() {
        return;
    }

    let fd = 0;
    let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
    if unsafe { libc::tcgetattr(fd, &mut termios) } != 0 {
        vm.runtime_error("internal_input_init failed to read terminal settings");
    }

    let orig_termios = termios;
    let mut raw_termios = termios;
    raw_termios.c_lflag &= !(libc::ICANON | libc::ECHO);
    raw_termios.c_cc[libc::VMIN] = 0;
    raw_termios.c_cc[libc::VTIME] = 0;

    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw_termios) } != 0 {
        vm.runtime_error("internal_input_init failed to set raw mode");
    }

    let orig_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if orig_flags < 0 {
        vm.runtime_error("internal_input_init failed to read file flags");
    }

    let raw_flags = orig_flags | libc::O_NONBLOCK;
    if unsafe { libc::fcntl(fd, libc::F_SETFL, raw_flags) } != 0 {
        vm.runtime_error("internal_input_init failed to set non-blocking mode");
    }

    *guard = Some(UnixInputState {
        fd,
        orig_termios,
        raw_termios,
        orig_flags,
        raw_flags,
        pending: VecDeque::new(),
    });

    unix_register_atexit();
}

#[cfg(unix)]
fn unix_input_shutdown(vm: &mut VM) {
    let mut guard = unix_state()
        .lock()
        .unwrap_or_else(|_| vm.runtime_error("input state lock poisoned"));
    let state = guard.take();
    if let Some(state) = state {
        if unsafe { libc::tcsetattr(state.fd, libc::TCSANOW, &state.orig_termios) } != 0 {
            vm.runtime_error("internal_input_shutdown failed to restore terminal settings");
        }
        if unsafe { libc::fcntl(state.fd, libc::F_SETFL, state.orig_flags) } != 0 {
            vm.runtime_error("internal_input_shutdown failed to restore file flags");
        }
    }
}

#[cfg(unix)]
fn unix_suspend_raw_input() -> Option<(i32, libc::termios, i32, libc::termios)> {
    let mut guard = unix_state().lock().ok()?;
    let state = guard.as_mut()?;
    let fd = state.fd;
    let orig_termios = state.orig_termios;
    let orig_flags = state.orig_flags;
    let raw_termios = state.raw_termios;
    let raw_flags = state.raw_flags;
    unsafe {
        libc::tcsetattr(fd, libc::TCSANOW, &orig_termios);
        libc::fcntl(fd, libc::F_SETFL, orig_flags);
    }
    Some((fd, raw_termios, raw_flags, orig_termios))
}

#[cfg(unix)]
fn unix_restore_raw_input(restore: Option<(i32, libc::termios, i32, libc::termios)>) {
    if let Some((fd, raw_termios, raw_flags, _orig_termios)) = restore {
        unsafe {
            libc::tcsetattr(fd, libc::TCSANOW, &raw_termios);
            libc::fcntl(fd, libc::F_SETFL, raw_flags);
        }
    }
}

#[cfg(unix)]
fn unix_input_poll(vm: &mut VM) -> i32 {
    let mut guard = unix_state()
        .lock()
        .unwrap_or_else(|_| vm.runtime_error("input state lock poisoned"));
    let state = guard
        .as_mut()
        .unwrap_or_else(|| vm.runtime_error("internal_input_poll called before input_init"));

    loop {
        let mut buf = [0u8; 32];
        let n = unsafe { libc::read(state.fd, buf.as_mut_ptr() as *mut _, buf.len()) };
        if n > 0 {
            state.pending.extend(&buf[..n as usize]);
            continue;
        }
        if n == 0 {
            break;
        }
        if n < 0 {
            let err = io::Error::last_os_error();
            let code = err.raw_os_error().unwrap_or(0);
            if code == libc::EAGAIN || code == libc::EWOULDBLOCK {
                break;
            }
            vm.runtime_error(&format!("internal_input_poll read failed: {err}"));
        }
    }

    if state.pending.is_empty() {
        return -1;
    }

    if state.pending[0] == 27 {
        if state.pending.len() >= 3 && state.pending[1] == b'[' {
            let code = match state.pending[2] {
                b'A' => KEY_UP,
                b'B' => KEY_DOWN,
                b'C' => KEY_RIGHT,
                b'D' => KEY_LEFT,
                _ => -1,
            };
            if code != -1 {
                state.pending.drain(..3);
                return code;
            }
        }

        if state.pending.len() >= 2 && state.pending[1] == b'[' {
            return -1;
        }

        state.pending.pop_front();
        return 27;
    }

    state.pending.pop_front().map(|b| b as i32).unwrap_or(-1)
}

#[cfg(windows)]
struct WindowsInputState {
    handle: HANDLE,
    orig_mode: u32,
    raw_mode: u32,
}

#[cfg(windows)]
fn win_state() -> &'static Mutex<Option<WindowsInputState>> {
    static STATE: OnceLock<Mutex<Option<WindowsInputState>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

#[cfg(windows)]
fn win_register_ctrl_handler() {
    static REGISTER: OnceLock<()> = OnceLock::new();
    if REGISTER.set(()).is_ok() {
        unsafe {
            SetConsoleCtrlHandler(Some(win_ctrl_handler), 1);
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn win_ctrl_handler(_ctrl_type: u32) -> BOOL {
    win_restore_state();
    0
}

#[cfg(windows)]
fn win_restore_state() {
    if let Some(state) = win_state().lock().ok().and_then(|mut guard| guard.take()) {
        unsafe {
            SetConsoleMode(state.handle, state.orig_mode);
        }
    }
}

#[cfg(windows)]
unsafe extern "C" {
    fn _kbhit() -> i32;
    fn _getch() -> i32;
}

#[cfg(windows)]
fn win_input_init(vm: &mut VM) {
    let mut guard = win_state()
        .lock()
        .unwrap_or_else(|_| vm.runtime_error("input state lock poisoned"));
    if guard.is_some() {
        return;
    }

    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if handle == INVALID_HANDLE_VALUE || handle == 0 {
        vm.runtime_error("internal_input_init failed to get stdin handle");
    }

    let mut orig_mode = 0u32;
    if unsafe { GetConsoleMode(handle, &mut orig_mode) } == 0 {
        vm.runtime_error("internal_input_init failed to read console mode");
    }

    let raw_mode = orig_mode & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT);
    if unsafe { SetConsoleMode(handle, raw_mode) } == 0 {
        vm.runtime_error("internal_input_init failed to set raw console mode");
    }

    *guard = Some(WindowsInputState {
        handle,
        orig_mode,
        raw_mode,
    });

    win_register_ctrl_handler();
}

#[cfg(windows)]
fn win_input_shutdown(vm: &mut VM) {
    let mut guard = win_state()
        .lock()
        .unwrap_or_else(|_| vm.runtime_error("input state lock poisoned"));
    let state = guard.take();
    if let Some(state) = state {
        if unsafe { SetConsoleMode(state.handle, state.orig_mode) } == 0 {
            vm.runtime_error("internal_input_shutdown failed to restore console mode");
        }
    }
}

#[cfg(windows)]
fn win_suspend_raw_input() -> Option<(HANDLE, u32)> {
    let mut guard = win_state().lock().ok()?;
    let state = guard.as_mut()?;
    unsafe {
        SetConsoleMode(state.handle, state.orig_mode);
    }
    Some((state.handle, state.raw_mode))
}

#[cfg(windows)]
fn win_restore_raw_input(restore: Option<(HANDLE, u32)>) {
    if let Some((handle, raw_mode)) = restore {
        unsafe {
            SetConsoleMode(handle, raw_mode);
        }
    }
}

#[cfg(windows)]
fn win_input_poll(vm: &mut VM) -> i32 {
    let guard = win_state()
        .lock()
        .unwrap_or_else(|_| vm.runtime_error("input state lock poisoned"));
    if guard.is_none() {
        vm.runtime_error("internal_input_poll called before input_init");
    }
    drop(guard);

    let available = unsafe { _kbhit() };
    if available == 0 {
        return -1;
    }

    let ch = unsafe { _getch() };
    if ch == 0 || ch == 224 {
        let code = unsafe { _getch() };
        return match code {
            72 => KEY_UP,
            80 => KEY_DOWN,
            75 => KEY_LEFT,
            77 => KEY_RIGHT,
            other => other as i32,
        };
    }

    ch as i32
}
