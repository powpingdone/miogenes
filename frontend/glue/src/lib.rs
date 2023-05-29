mod bridge_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

mod api;

// i dunno why this many worker_threads
pub(crate) static RUNTIME: Lazy<Runtime> =
    Lazy::new(
        || tokio::runtime::Builder::new_multi_thread()
            .worker_threads((num_cpus::get_physical() / 2).min(1))
            .enable_all()
            .build()
            .unwrap(),
    );
