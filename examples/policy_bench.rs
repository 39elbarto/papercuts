use papercuts::policy::{SensitiveCategory, SensitivePolicy};
use papercuts::sensitive::preflight_add;
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000)
        .max(10_000);
    let prefix = concat!(
        "-----BEGIN ",
        "PRIVATE KEY-----\n",
        "Authorization: Bearer SyntheticBearerMaterial99\n",
        "https://synthetic-user:synthetic-pass@example.invalid/db\n",
        "password=synthetic-literal-value\n",
        "ghp_SyntheticTokenBody99\n",
        "xoxb-synthetic-token-body-99\n",
        "sk_",
        "test_SyntheticKeyBody99\n",
        "AKIA",
        "ABCDEFGHIJKLMNOP\n",
        "AWS_SECRET_ACCESS_KEY=synthetic-secret-material\n",
        "alice@example.invalid\n",
        "customer_id=customer-2048\n",
        "/data/projects/example/file.txt\n",
        "alpha=one\n",
        "beta=two\n",
    );
    let mut text = prefix.to_owned();
    text.push_str(&"x".repeat(10_000 - text.len()));
    let tags = vec!["t".repeat(64); 16];
    let agent = "a".repeat(128);

    let warmup = preflight_add(
        SensitivePolicy::Strict,
        &SensitiveCategory::ALL,
        &text,
        &tags,
        &agent,
    )
    .expect("the benchmark corpus must exercise every category");
    let mut expected_categories = SensitiveCategory::ALL.to_vec();
    expected_categories.sort_by_key(|category| category.as_str());
    assert_eq!(warmup.categories, expected_categories);

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        black_box(
            preflight_add(
                SensitivePolicy::Strict,
                &SensitiveCategory::ALL,
                black_box(&text),
                black_box(&tags),
                black_box(&agent),
            )
            .expect("the fixed benchmark corpus must remain accepted by exact override"),
        );
        samples.push(started.elapsed().as_nanos() as u64);
    }
    samples.sort_unstable();
    let percentile = |percent: usize| samples[(samples.len() - 1) * percent / 100];
    println!(
        "{}",
        json!({
            "iterations": iterations,
            "payload_bytes": text.len() + tags.iter().map(String::len).sum::<usize>() + agent.len(),
            "patterns": papercuts::sensitive::PATTERN_COUNT,
            "policy_version": papercuts::sensitive::POLICY_VERSION,
            "host": std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            "commit": std::env::var("PAPERCUTS_BENCH_COMMIT").unwrap_or_else(|_| "working-tree".into()),
            "binary_sha256": std::env::var("PAPERCUTS_BENCH_BINARY_SHA256").unwrap_or_else(|_| "unrecorded".into()),
            "p50_ns": percentile(50),
            "p95_ns": percentile(95),
            "max_ns": *samples.last().unwrap(),
        })
    );
}
