## 2024-05-18 - [PID Validation Already Implemented]
**Vulnerability:** Potential negative PID integer wrap leading to unintended process targeting via libc functions.
**Learning:** Found that this codebase explicitly implements PID validation checks (e.g. `pid == 0 || pid > i32::MAX as u32`) and file permission checks (e.g. `O_NOFOLLOW` with `0o600` mode) prior to interacting with external systems via FFI or creating temporary files.
**Prevention:** Always verify via `grep` or file reads that expected common vulnerabilities aren’t already mitigated in the repository before planning a fix.
