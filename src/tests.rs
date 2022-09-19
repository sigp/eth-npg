use futures::StreamExt;

use super::*;

#[tokio::test]
async fn test_generator() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder().finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let mut g = Generator::<SystemTimeSlotClock, String>::new(
        Slot::new(0),
        Duration::ZERO,
        Duration::from_secs(2),
        3, //node_id
        5, //total_nodes
    );
    while let Some(m) = g.next().await {
        println!("{m}")
    }
}
