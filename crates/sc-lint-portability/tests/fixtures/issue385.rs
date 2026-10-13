pub fn issue385_paths() {
    let unix_cache = "/home/example/.cache/sc-lint";
    let windows_cache = r"C:\\ProgramData\\sc-lint\\cache";
    let _ = (unix_cache, windows_cache);
}
