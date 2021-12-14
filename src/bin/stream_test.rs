use clap::Parser;
use stabilizer_streaming::StreamReceiver;
use std::time::{Duration, Instant};

const MAX_LOSS: f32 = 0.05;

/// Execute stabilizer stream throughput testing.
/// Use `RUST_LOG=info cargo run` to increase logging verbosity.
#[derive(Parser)]
struct Opts {
    /// The local IP to receive streaming data on.
    #[clap(short, long, default_value = "0.0.0.0")]
    ip: String,

    /// The UDP port to receive streaming data on.
    #[clap(long, default_value = "9293")]
    port: u16,

    /// The test duration in seconds.
    #[clap(long, default_value = "5")]
    duration: f32,
}

#[async_std::main]
async fn main() {
    env_logger::init();

    let opts = Opts::parse();
    let ip: std::net::Ipv4Addr = opts.ip.parse().unwrap();

    log::info!("Binding to socket");
    let mut stream_receiver = StreamReceiver::new(ip, opts.port).await;
    stream_receiver.set_timeout(Duration::from_secs(1));

    let mut total_batches = 0u64;
    let mut dropped_batches = 0u64;
    let mut expect_sequence = None;

    let stop = Instant::now() + Duration::from_millis((opts.duration * 1000.) as _);

    log::info!("Reading frames");
    while Instant::now() < stop {
        let frame = stream_receiver.next_frame().await.unwrap();
        total_batches += frame.data.batch_count() as u64;

        if let Some(expect) = expect_sequence {
            let num_dropped = frame.sequence_number().wrapping_sub(expect) as u64;
            dropped_batches += num_dropped;
            total_batches += num_dropped;

            if num_dropped > 0 {
                log::warn!(
                    "Lost {} batches: {:#08X} -> {:#08X}",
                    num_dropped,
                    expect,
                    frame.sequence_number(),
                );
            }
        }

        expect_sequence = Some(
            frame
                .sequence_number()
                .wrapping_add(frame.data.batch_count() as _),
        );
    }

    assert!(total_batches > 0);
    let loss = dropped_batches as f32 / total_batches as f32;

    log::info!(
        "Loss: {} % ({}/{} batches)",
        loss * 100.0,
        dropped_batches,
        total_batches
    );

    assert!(loss < MAX_LOSS);
}
