#[tokio::main]
async fn main() {
    let response = reqwest::get("http://example.org").await.unwrap();
    println!(
        "Got: HTTP {}, with headers: {:#?}",
        response.status(),
        response.headers(),
    );

    let body = response.text().await.unwrap();

    let num_lines = 10;
    println!("First {num_lines} lines of the body:");
    for line in body.lines().take(num_lines) {
        println!("{line}");
    }
}
