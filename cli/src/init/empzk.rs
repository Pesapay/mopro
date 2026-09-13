use crate::init::adapter::Adapter;
use crate::init::proving_system::ProvingSystem;
use include_dir::include_dir;
use include_dir::Dir;

pub struct Empzk;

impl ProvingSystem for Empzk {
    const TEMPLATE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/src/template/empzk");

    const ADAPTER: Adapter = Adapter::Empzk;

    // emp-zk (emp-toolkit Wolverine/QuickSilver) built from pinned sources with vendored OpenSSL.
    const DEPENDENCIES: &'static str = r#"
emp-zk-mopro = { git = "https://github.com/Pesapay/emp-zk-mopro", rev = "f21e07e3889f0a2b195d7e1e2a9aa55ffb97428d" }
    "#;
}
