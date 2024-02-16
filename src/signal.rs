use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    sync::watch,
};

pub(crate) fn signal_handler() -> watch::Receiver<()> {
    let (stop_tx, stop_rx) = watch::channel(());
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        loop {
            select! {
                _ = sigterm.recv() => println!("Receive SIGTERM"),
                _ = sigint.recv() => println!("Receive SIGTERM"),
            };
            stop_tx.send(()).unwrap();
        }
    });
    stop_rx
}
