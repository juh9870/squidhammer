#![allow(unreachable_pub)]

xflags::xflags! {
    cmd xtask {
        /// Runs a dev version of EHCE, using bevy dynamic_linking
        cmd dev {}
        /// Runs a code watcher
        cmd watch {}
        /// Runs all configured linters
        cmd fix {}
    }
}
