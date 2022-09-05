pub(crate) fn alloc_console() {
    // 手动初始化一个控制台，参考：https://stackoverflow.com/a/191880/17353788

    extern "C" {
        fn __acrt_iob_func(fileno: u32) -> *mut libc::FILE;
    }

    use std::{mem::MaybeUninit, ptr::null_mut};

    use windows_sys::Win32::System::Console::{
        AllocConsole, GetConsoleScreenBufferInfo, GetStdHandle, SetConsoleScreenBufferSize,
        STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    const MAX_CONSOLE_LINES: i16 = 500;

    let mut coninfo = MaybeUninit::zeroed();

    unsafe {
        let stdin = __acrt_iob_func(0);
        let stdout = __acrt_iob_func(1);
        let stderr = __acrt_iob_func(2);

        let mut coninfo = coninfo.assume_init_mut();

        AllocConsole();
        GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), coninfo);
        coninfo.dwSize.Y = MAX_CONSOLE_LINES;
        SetConsoleScreenBufferSize(GetStdHandle(STD_OUTPUT_HANDLE), coninfo.dwSize);

        let std_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let con_handle = libc::open_osfhandle(std_handle, libc::O_TEXT);
        let fp = libc::fdopen(con_handle, "w\0".as_ptr() as _);
        *stdout = *fp;
        libc::setvbuf(stdout, null_mut(), libc::_IONBF, 0);

        let std_handle = GetStdHandle(STD_INPUT_HANDLE);
        let con_handle = libc::open_osfhandle(std_handle, libc::O_TEXT);
        let fp = libc::fdopen(con_handle, "r\0".as_ptr() as _);
        *stdin = *fp;
        libc::setvbuf(stdin, null_mut(), libc::_IONBF, 0);

        let std_handle = GetStdHandle(STD_ERROR_HANDLE);
        let con_handle = libc::open_osfhandle(std_handle, libc::O_TEXT);
        let fp = libc::fdopen(con_handle, "w\0".as_ptr() as _);
        *stderr = *fp;
        libc::setvbuf(stderr, null_mut(), libc::_IONBF, 0);

        // make cout, wcout, cin, wcin, wcerr, cerr, wclog and clog
        // point to console as well
        // ios::sync_with_stdio();
    }
}
