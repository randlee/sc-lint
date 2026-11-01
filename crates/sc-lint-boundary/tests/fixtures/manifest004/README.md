`atm_core_6ffaa97be_commands_mod.rs` is a verbatim copy of atm-core
`crates/atm/src/commands/mod.rs` at commit
`6ffaa97be3fb17db3f0f96c48e8d662d52e92f63` (issue #136). It declares
`pub mod cron;` and names `Cron(self::cron::CronCommand)`, a local module path
that SCB-MANIFEST-004 reported as a use of the owned `cron` crate.
