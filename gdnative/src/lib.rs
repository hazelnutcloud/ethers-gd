mod frame_provider;

use frame_provider::FrameProvider;
use gdnative::prelude::*;
use tokio::{task::LocalSet, runtime::{Runtime, Builder}};

thread_local! {
    static EXECUTOR: &'static SharedLocalPool = {
        Box::leak(Box::new(SharedLocalPool::default()))
    };
}

#[derive(Default)]
struct SharedLocalPool {
    local_set: LocalSet
}

impl futures::task::LocalSpawn for SharedLocalPool {
    fn spawn_local_obj(
        &self,
        future: futures::task::LocalFutureObj<'static, ()>,
    ) -> Result<(), futures::task::SpawnError> {
        self.local_set.spawn_local(future);

        Ok(())
    }
}

#[derive(NativeClass)]
#[inherit(Node)]
struct AsyncExecutorDriver {
    runtime: Runtime,
}

impl AsyncExecutorDriver {
    fn new(_owner: &Node) -> Self {
        AsyncExecutorDriver {
            runtime: Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
        }
    }
}

#[methods]
impl AsyncExecutorDriver {
    #[export]
    fn _process(&self, _owner: &Node, _delta: f64) {
        EXECUTOR.with(|e| {
            self.runtime
                .block_on(async {
                    e.local_set
                        .run_until(async {
                            tokio::task::spawn_local(async {}).await
                        })
                        .await
                })
                .unwrap()
        });
    }
}

/// register classes
fn init(handle: InitHandle) {
    gdnative::tasks::register_runtime(&handle);
    gdnative::tasks::set_executor(EXECUTOR.with(|e| *e));

    handle.add_class::<AsyncExecutorDriver>();
    handle.add_class::<FrameProvider>();
}

godot_init!(init);
