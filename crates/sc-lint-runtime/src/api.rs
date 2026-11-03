//! Canonical runtime API paths. Aliases, type inference and rule dispatch all
//! consult this registry; glob eligibility preserves the practices resolver's
//! existing subset rather than widening legacy rule 003's recognition policy.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Api {
    Std,
    Thread,
    ThreadSleep,
    ThreadSleepUntil,
    Mutex,
    MutexGuard,
    MutexNew,
    MutexLock,
    Drop,
    TokioSleep,
    TokioSleepUntil,
    Runtime,
    Handle,
    Builder,
    RuntimeNew,
    HandleCurrent,
    BuilderNew,
    BlockOn,
    Owner,
    OwnerConstructor,
}

struct Entry {
    path: &'static str,
    api: Api,
    glob: bool,
}

impl Entry {
    const fn direct(path: &'static str, api: Api) -> Self {
        Self {
            path,
            api,
            glob: false,
        }
    }
    const fn glob(path: &'static str, api: Api) -> Self {
        Self {
            path,
            api,
            glob: true,
        }
    }
}

const PATHS: &[Entry] = &[
    Entry::direct("std", Api::Std),
    Entry::direct("std::thread", Api::Thread),
    Entry::direct("std::thread::sleep", Api::ThreadSleep),
    Entry::glob("std::thread::sleep_until", Api::ThreadSleepUntil),
    Entry::glob("std::sync::Mutex", Api::Mutex),
    Entry::glob("std::sync::MutexGuard", Api::MutexGuard),
    Entry::direct("std::sync::Mutex::new", Api::MutexNew),
    Entry::direct("std::sync::Mutex::lock", Api::MutexLock),
    Entry::direct("std::sync::Mutex::try_lock", Api::MutexLock),
    Entry::glob("std::mem::drop", Api::Drop),
    Entry::direct("drop", Api::Drop),
    Entry::glob("tokio::time::sleep", Api::TokioSleep),
    Entry::glob("tokio::time::sleep_until", Api::TokioSleepUntil),
    Entry::glob("tokio::runtime::Runtime", Api::Runtime),
    Entry::glob("tokio::runtime::Handle", Api::Handle),
    Entry::glob("tokio::runtime::Builder", Api::Builder),
    Entry::direct("tokio::runtime::Runtime::new", Api::RuntimeNew),
    Entry::direct("tokio::runtime::Handle::current", Api::HandleCurrent),
    Entry::direct("tokio::runtime::Handle::try_current", Api::HandleCurrent),
    Entry::direct(
        "tokio::runtime::Builder::new_current_thread",
        Api::BuilderNew,
    ),
    Entry::direct("tokio::runtime::Builder::new_multi_thread", Api::BuilderNew),
    Entry::direct("tokio::runtime::Runtime::block_on", Api::BlockOn),
    Entry::direct("tokio::runtime::Handle::block_on", Api::BlockOn),
    Entry::glob("futures::executor::block_on", Api::BlockOn),
    Entry::direct("Arc", Api::Owner),
    Entry::direct("Arc::new", Api::OwnerConstructor),
    Entry::direct("Arc::clone", Api::OwnerConstructor),
    Entry::direct("Rc", Api::Owner),
    Entry::direct("Rc::new", Api::OwnerConstructor),
    Entry::direct("Rc::clone", Api::OwnerConstructor),
    Entry::direct("Box", Api::Owner),
    Entry::direct("Box::new", Api::OwnerConstructor),
    Entry::glob("std::sync::Arc", Api::Owner),
    Entry::direct("std::sync::Arc::new", Api::OwnerConstructor),
    Entry::direct("std::sync::Arc::clone", Api::OwnerConstructor),
    Entry::glob("std::rc::Rc", Api::Owner),
    Entry::direct("std::rc::Rc::new", Api::OwnerConstructor),
    Entry::direct("std::rc::Rc::clone", Api::OwnerConstructor),
    Entry::direct("std::boxed::Box", Api::Owner),
    Entry::direct("std::boxed::Box::new", Api::OwnerConstructor),
    Entry::direct("alloc::sync::Arc", Api::Owner),
    Entry::direct("alloc::rc::Rc", Api::Owner),
    Entry::direct("alloc::boxed::Box", Api::Owner),
];

pub(crate) fn lookup(path: &str) -> Option<Api> {
    PATHS
        .iter()
        .find(|entry| entry.path == path)
        .map(|entry| entry.api)
}

pub(crate) fn known_glob_path(path: &[String]) -> bool {
    let path = path.join("::");
    PATHS.iter().any(|entry| entry.glob && entry.path == path)
}
