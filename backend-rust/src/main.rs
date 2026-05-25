#[tokio::main]
async fn main() {
    genshin_gacha_api::init_tracing();
    let addr = "127.0.0.1:8001";
    if let Err(e) = genshin_gacha_api::run_server(addr).await {
        eprintln!("服务启动失败: {e}");
        std::process::exit(1);
    }
}
