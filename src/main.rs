use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use std::fs;
use std::process::Command;

// 镜像源配置
const MIRROR_URL: &str = "https://hub.fastgit.xyz";
const MIRROR_URL_BACKUP: &str = "https://gitclone.com";

// 测速端点
const GITHUB_TEST_URL: &str = "https://github.com";

#[derive(Parser)]
#[command(name = "jarvis")]
#[command(author = "Jiannei")]
#[command(version = "0.1.0")]
#[command(about = "GitHub 访问加速工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 直连模式 - 优化 DNS/hosts
    Direct,
    /// 镜像模式 - 配置 Git 镜像加速
    Mirror,
    /// 代理模式 - 配置 Git 代理
    Proxy {
        /// 代理服务器地址 (例如：127.0.0.1:7890)
        #[arg(short, long)]
        addr: Option<String>,
    },
    /// SSH 模式 - 配置 SSH 密钥
    Ssh,
    /// 测试 GitHub 连接
    Test,
    /// 查看当前配置
    Config,
    /// 自动测速 - 选择最快的方式
    SpeedTest,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Direct) => optimize_dns(),
        Some(Commands::Mirror) => config_git_mirror(),
        Some(Commands::Proxy { addr }) => config_proxy(addr.clone()),
        Some(Commands::Ssh) => setup_ssh(),
        Some(Commands::Test) => test_connection(),
        Some(Commands::Config) => show_config(),
        Some(Commands::SpeedTest) => auto_speed_test(),
        None => interactive_mode(),
    }
}

fn interactive_mode() {
    println!("{}", "========================================".blue());
    println!("{}", "   GitHub 访问加速工具".blue());
    println!("{}", "========================================".blue());
    println!();

    loop {
        let choices = vec![
            "直连模式 - 优化 DNS/hosts",
            "镜像模式 - 配置 Git 镜像加速",
            "代理模式 - 配置 Git 代理 (需要代理服务器)",
            "SSH 模式 - 配置 SSH 密钥",
            "测试 GitHub 连接",
            "查看当前配置",
            "自动测速 - 选择最快的方式",
            "恢复默认并退出",
        ];

        let selection = Select::new()
            .with_prompt("请选择")
            .items(&choices)
            .interact()
            .unwrap();

        match selection {
            0 => optimize_dns(),
            1 => config_git_mirror(),
            2 => config_proxy(None),
            3 => setup_ssh(),
            4 => test_connection(),
            5 => show_config(),
            6 => auto_speed_test(),
            7 => {
                restore_default();
                println!("{}", "退出".green());
                std::process::exit(0);
            }
            _ => println!("无效选项"),
        }

        println!();
    }
}

fn resolve_github_hosts() {
    println!("{}", "正在查询 GitHub 相关域名 IP...".yellow());

    let domains = vec![
        "github.com",
        "api.github.com",
        "raw.githubusercontent.com",
        "gist.githubusercontent.com",
        "cloud.githubusercontent.com",
        "camo.githubusercontent.com",
        "avatars.githubusercontent.com",
        "github.githubassets.com",
        "github-cloud.s3.amazonaws.com",
    ];

    println!();
    println!("{}", "建议添加到 /etc/hosts 的记录:".blue());
    println!("{}", "# GitHub Start".white());

    for domain in domains {
        if let Ok(output) = Command::new("dig").arg("+short").arg(domain).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let ip = stdout.lines().next().unwrap_or("").trim();
            if !ip.is_empty() {
                println!("{} {}", ip, domain);
                continue;
            }
        }

        // 备用：使用公共 DNS 查询
        if let Ok(output) = Command::new("nslookup")
            .arg(domain)
            .arg("8.8.8.8")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("Address:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() == 2 {
                        println!("{} {}", parts[1].trim(), domain);
                        break;
                    }
                }
            }
        }
    }

    println!("{}", "# GitHub End".white());
    println!();
}

fn backup_hosts() -> String {
    let now = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let backup_file = format!("/etc/hosts.backup.{}", now);
    println!("{}", format!("备份 hosts 文件到：{}", backup_file).yellow());

    Command::new("sudo")
        .arg("cp")
        .arg("/etc/hosts")
        .arg(&backup_file)
        .status()
        .expect("备份 hosts 失败");

    backup_file
}

fn optimize_dns() {
    println!("{}", "此操作需要 sudo 权限".yellow());
    println!();

    // 先显示建议的 hosts 记录
    resolve_github_hosts();

    let confirm = Confirm::new()
        .with_prompt("是否自动添加到 /etc/hosts?")
        .default(false)
        .interact()
        .unwrap();

    if !confirm {
        println!("已取消");
        return;
    }

    // 备份
    backup_hosts();

    println!("{}", "测试 IP 延迟，选择最快的...".yellow());

    // 获取延迟最低的 IP
    let (github_ip, github_latency) = resolve_with_latency("github.com");
    let (api_ip, api_latency) = resolve_with_latency("api.github.com");
    let (raw_ip, raw_latency) = resolve_with_latency("raw.githubusercontent.com");

    println!("  github.com: {} ({}ms)", github_ip, github_latency.unwrap_or(-1));
    println!("  api.github.com: {} ({}ms)", api_ip, api_latency.unwrap_or(-1));
    println!("  raw.githubusercontent.com: {} ({}ms)", raw_ip, raw_latency.unwrap_or(-1));
    println!();

    // 移除旧的 GitHub 记录
    let _ = Command::new("sudo")
        .arg("sed")
        .arg("-i.bak")
        .arg("/# GitHub Start/,/# GitHub End/d")
        .arg("/etc/hosts")
        .status();

    // 添加新记录
    let hosts_content = format!(
        r#"
# GitHub Start
{} github.com
{} www.github.com
{} api.github.com
{} raw.githubusercontent.com
{} gist.githubusercontent.com
{} cloud.githubusercontent.com
{} camo.githubusercontent.com
{} avatars.githubusercontent.com
{} github.githubassets.com
# GitHub End
"#,
        github_ip, github_ip, api_ip, raw_ip, raw_ip, raw_ip, raw_ip, raw_ip, raw_ip
    );

    Command::new("sudo")
        .arg("sh")
        .arg("-c")
        .arg(format!("cat >> /etc/hosts << EOF\n{}\nEOF", hosts_content))
        .status()
        .expect("写入 hosts 失败");

    // 刷新 DNS 缓存
    println!("{}", "刷新 DNS 缓存...".yellow());
    let _ = Command::new("sudo").arg("dscacheutil").arg("-flushcache").status();
    let _ = Command::new("sudo").arg("killall").arg("-HUP").arg("mDNSResponder").status();

    println!("{}", "✓ DNS 优化完成".green());
    println!();
    println!(
        "{}",
        "提示：如果效果不佳，可以手动访问 https://github.com.ipaddress.com 获取最新 IP".yellow()
    );
}

fn test_ip_latency(ip: &str, timeout: u64) -> Option<i64> {
    if let Ok(output) = Command::new("ping")
        .args(["-c", "1", "-W", &timeout.to_string(), ip])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("rtt") || line.contains("round-trip") {
                let parts: Vec<&str> = line.split('/').collect();
                if parts.len() >= 5 {
                    if let Ok(ms) = parts[4].trim().parse::<f64>() {
                        return Some(ms as i64);
                    }
                }
            }
        }
    }
    None
}

fn resolve_with_latency(domain: &str) -> (String, Option<i64>) {
    // 先用 dig 获取当前解析的 IP 列表
    let mut ips: Vec<String> = Vec::new();

    if let Ok(output) = Command::new("dig").arg("+short").arg(domain).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let ip = line.trim();
            if !ip.is_empty() && ip.contains('.') {
                ips.push(ip.to_string());
            }
        }
    }

    // 如果 dig 没获取到，使用备用 IP 池
    if ips.is_empty() {
        ips = match domain {
            "github.com" => vec!["140.82.112.3", "140.82.113.3", "140.82.114.3"].into_iter().map(String::from).collect(),
            "api.github.com" => vec!["140.82.112.5", "140.82.113.5", "140.82.114.5"].into_iter().map(String::from).collect(),
            "raw.githubusercontent.com" => vec!["185.199.108.133", "185.199.109.133", "185.199.110.133", "185.199.111.133"].into_iter().map(String::from).collect(),
            _ => vec![],
        };
    }

    println!("  {} 可选 IP: {:?}", domain, ips);

    let mut best_ip = ips.first().unwrap().to_string();
    let mut best_latency: Option<i64> = None;

    for ip in &ips {
        if let Some(latency) = test_ip_latency(ip, 2) {
            if best_latency.is_none() || latency < best_latency.unwrap() {
                best_ip = ip.to_string();
                best_latency = Some(latency);
            }
        }
    }

    (best_ip, best_latency)
}

fn config_git_mirror() {
    println!("{}", "配置 Git 镜像加速（URL 重写模式）".yellow());
    println!();
    println!("镜像源：{}", MIRROR_URL.white());
    println!();
    println!(
        "{}",
        "此配置会将所有 github.com 的请求重定向到镜像源".white()
    );
    println!(
        "{}",
        "启用后，执行 git clone/pull/push 时自动使用镜像".white()
    );
    println!();

    let choices = vec![
        "启用镜像模式",
        "禁用镜像模式",
        "查看镜像配置状态",
        "返回",
    ];

    let selection = Select::new()
        .with_prompt("选择")
        .items(&choices)
        .interact()
        .unwrap();

    match selection {
        0 => {
            println!("{}", "启用镜像模式...".blue());
            // 先清理可能存在的旧配置
            let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL)]);
            let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL_BACKUP)]);

            // 配置新的镜像
            let _ = run_git(&["config", "--global", &format!("url.{}/.insteadOf", MIRROR_URL), "https://github.com/"]);
            let _ = run_git(&["config", "--global", &format!("url.{}/.insteadOf", MIRROR_URL), "git://github.com/"]);

            println!("{}", "✓ 镜像模式已启用".green());
            println!();
            println!("使用方法：");
            println!("  git clone https://github.com/user/repo.git  # 自动走镜像");
        }
        1 => {
            println!("{}", "禁用镜像模式...".blue());
            let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL)]);
            let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL_BACKUP)]);
            println!("{}", "✓ 镜像模式已禁用".green());
        }
        2 => {
            println!("{}", "当前镜像配置：".blue());
            let output = run_git(&["config", "--global", "--get-regexp", "url\\..*\\.insteadOf"]);
            if output.is_empty() {
                println!("  未配置镜像");
            } else {
                println!("{}", output);
            }
        }
        _ => {}
    }
}

fn run_git(args: &[&str]) -> String {
    if let Ok(output) = Command::new("git").args(args).output() {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    String::new()
}

fn config_proxy(addr: Option<String>) {
    println!("{}", "配置 Git 代理...".yellow());
    println!();

    let proxy_addr = if let Some(addr) = addr {
        addr
    } else {
        Input::new()
            .with_prompt("输入代理服务器地址 (例如：127.0.0.1:7890)")
            .interact_text()
            .unwrap()
    };

    if proxy_addr.is_empty() {
        println!("已取消");
        return;
    }

    let _ = run_git(&["config", "--global", "http.proxy", &proxy_addr]);
    let _ = run_git(&["config", "--global", "https.proxy", &proxy_addr]);

    println!("{}", "✓ Git 代理配置完成".green());
    println!();
    println!("取消代理:");
    println!("  git config --global --unset http.proxy");
    println!("  git config --global --unset https.proxy");
}

fn setup_ssh() {
    println!("{}", "配置 SSH 密钥...".yellow());
    println!();

    let home = shellexpand::tilde("~");
    let ed25519_key = format!("{}/.ssh/id_ed25519", home);
    let rsa_key = format!("{}/.ssh/id_rsa", home);

    if fs::metadata(&ed25519_key).is_ok() {
        println!(
            "{}",
            format!("✓ 找到 Ed25519 密钥：{}", ed25519_key).green()
        );
    } else if fs::metadata(&rsa_key).is_ok() {
        println!(
            "{}",
            format!("✓ 找到 RSA 密钥：{}", rsa_key).green()
        );
    } else {
        println!("{}", "未找到 SSH 密钥".yellow());

        let confirm = Confirm::new()
            .with_prompt("是否生成新的 SSH 密钥？")
            .default(false)
            .interact()
            .unwrap();

        if confirm {
            println!("生成 Ed25519 密钥...");
            let _ = Command::new("ssh-keygen")
                .args(["-t", "ed25519", "-C", "github"])
                .status();

            println!();
            println!("公钥内容：");
            if let Ok(content) = fs::read_to_string(format!("{}/.ssh/id_ed25519.pub", home)) {
                println!("{}", content);
            }
            println!();
            println!("请复制以上公钥到 GitHub SSH Keys 设置：");
            println!("https://github.com/settings/keys");
        }
        return;
    }

    // 测试 SSH 连接
    println!();
    println!("测试 SSH 连接...");
    let _ = Command::new("ssh")
        .args(["-T", "-o", "ConnectTimeout=5", "git@github.com"])
        .status();
}

fn test_connection() {
    println!("{}", "测试 GitHub 连接...".yellow());
    println!();

    println!("{}", "1. DNS 解析测试:".blue());
    if let Ok(output) = Command::new("dig").arg("+short").arg("github.com").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().take(3) {
            println!("{}", line);
        }
    }
    println!();

    println!("{}", "2. 连接延迟测试:".blue());
    test_curl_latency("github.com", "https://github.com");
    test_curl_latency("api.github.com", "https://api.github.com");
    println!();

    println!("{}", "3. Git SSH 测试:".blue());
    let home = shellexpand::tilde("~");
    if fs::metadata(format!("{}/.ssh/id_ed25519.pub", home)).is_ok()
        || fs::metadata(format!("{}/.ssh/id_rsa.pub", home)).is_ok()
    {
        let _ = Command::new("ssh")
            .args(["-T", "-o", "ConnectTimeout=5", "git@github.com"])
            .status();
    } else {
        println!("未找到 SSH 密钥");
    }
    println!();

    println!("{}", "4. Git HTTPS 测试:".blue());
    let _ = Command::new("git")
        .args(["ls-remote", "--symref", "https://github.com/git/git.git", "HEAD"])
        .status();
}

fn test_curl_latency(name: &str, url: &str) {
    if let Ok(output) = Command::new("curl")
        .args([
            "-o",
            "/dev/null",
            "-s",
            "-w",
            "%{time_total}",
            "--connect-timeout",
            "10",
            url,
        ])
        .output()
    {
        let time = String::from_utf8_lossy(&output.stdout);
        println!("{}: {}s", name, time);
    } else {
        println!("{}: 连接超时", name);
    }
}

fn show_config() {
    println!("{}", "当前 Git 配置:".yellow());
    println!();

    println!("{}", "镜像配置:".blue());
    let mirror_config = run_git(&["config", "--global", "--get-regexp", "url\\..*\\.insteadOf"]);
    if mirror_config.is_empty() {
        println!("  未配置镜像");
    } else {
        println!("{}", mirror_config);
    }
    println!();

    println!("{}", "代理配置:".blue());
    let http_proxy = run_git(&["config", "--global", "http.proxy"]);
    let https_proxy = run_git(&["config", "--global", "https.proxy"]);
    if http_proxy.is_empty() {
        println!("  未配置 HTTP 代理");
    } else {
        println!("  HTTP 代理：{}", http_proxy);
    }
    if https_proxy.is_empty() {
        println!("  未配置 HTTPS 代理");
    } else {
        println!("  HTTPS 代理：{}", https_proxy);
    }
    println!();

    println!("{}", "hosts 配置:".blue());
    if let Ok(content) = fs::read_to_string("/etc/hosts") {
        let github_lines: Vec<&str> = content
            .lines()
            .filter(|line| line.contains("github.com"))
            .take(5)
            .collect();
        if github_lines.is_empty() {
            println!("  未配置 hosts");
        } else {
            for line in github_lines {
                println!("  {}", line);
            }
        }
    } else {
        println!("  无法读取 hosts 文件");
    }
    println!();

    println!("{}", "SSH 密钥:".blue());
    let home = shellexpand::tilde("~");
    if fs::metadata(format!("{}/.ssh/id_ed25519", home)).is_ok() {
        println!("  Ed25519: ✓");
    } else if fs::metadata(format!("{}/.ssh/id_rsa", home)).is_ok() {
        println!("  RSA: ✓");
    } else {
        println!("  未配置");
    }
}

fn test_latency(url: &str, timeout: u64) -> Option<i64> {
    if let Ok(output) = Command::new("curl")
        .args([
            "-o",
            "/dev/null",
            "-s",
            "-w",
            "%{time_total}",
            "--connect-timeout",
            &timeout.to_string(),
            url,
        ])
        .output()
    {
        let time_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(time) = time_str.parse::<f64>() {
            if time > 0.0 {
                return Some((time * 1000.0) as i64);
            }
        }
    }
    None
}

fn auto_speed_test() {
    println!("{}", "开始自动测速...".yellow());
    println!();

    // 显示当前配置
    println!("{}", "当前配置状态:".blue());
    let mirror_config = run_git(&["config", "--global", "--get-regexp", "url\\..*\\.insteadOf"]);
    if mirror_config.is_empty() {
        println!("  镜像配置：未配置");
    } else {
        println!("  镜像配置：{}", mirror_config);
    }

    let http_proxy = run_git(&["config", "--global", "http.proxy"]);
    if http_proxy.is_empty() {
        println!("  代理配置：未配置");
    } else {
        println!("  代理配置：{}", http_proxy);
    }

    let hosts_configured = fs::read_to_string("/etc/hosts")
        .map(|c| c.contains("github.com"))
        .unwrap_or(false);
    println!(
        "  hosts 配置：{}",
        if hosts_configured { "已配置" } else { "未配置" }
    );
    println!();

    let mut results: Vec<(String, String, i64, String)> = Vec::new();

    // 测试直连
    println!("{}", "测试直连...".blue());
    if let Some(latency) = test_latency(GITHUB_TEST_URL, 5) {
        println!("  直连：{}ms", latency);
        results.push(("直连".to_string(), "".to_string(), latency, "direct".to_string()));
    } else {
        println!("  直连：超时/失败");
    }

    // 测试镜像 1
    println!("{}", "测试镜像 1 (hub.fastgit.xyz)...".blue());
    let mirror1_url = format!("{}/{}", MIRROR_URL, "https://github.com");
    if let Some(latency) = test_latency(&mirror1_url, 5) {
        println!("  镜像 1: {}ms", latency);
        results.push((
            "镜像 (fastgit)".to_string(),
            MIRROR_URL.to_string(),
            latency,
            format!("mirror:{}", MIRROR_URL),
        ));
    } else {
        println!("  镜像 1: 超时/失败");
    }

    // 测试镜像 2
    println!("{}", "测试镜像 2 (gitclone.com)...".blue());
    let mirror2_url = "https://gitclone.com/github.com/git/git.git";
    if let Some(latency) = test_latency(mirror2_url, 5) {
        println!("  镜像 2: {}ms", latency);
        results.push((
            "镜像 (gitclone)".to_string(),
            MIRROR_URL_BACKUP.to_string(),
            latency,
            format!("mirror:{}", MIRROR_URL_BACKUP),
        ));
    } else {
        println!("  镜像 2: 超时/失败");
    }

    // 测试代理（如果已配置）
    let proxy_addr = run_git(&["config", "--global", "http.proxy"]);
    if !proxy_addr.is_empty() {
        println!("{}", format!("测试代理 ({})...", proxy_addr).blue());
        if let Some(latency) = test_latency(GITHUB_TEST_URL, 5) {
            println!("  代理：{}ms", latency);
            results.push((
                "代理".to_string(),
                proxy_addr.clone(),
                latency,
                format!("proxy:{}", proxy_addr),
            ));
        } else {
            println!("  代理：超时/失败");
        }
    }

    println!();

    if results.is_empty() {
        println!("{}", "所有测试都失败了，无法选择最优方案".red());
        return;
    }

    // 找出最快的
    let min_result = results.iter().min_by_key(|r| r.2).unwrap();

    println!("{}", "=========================================".green());
    println!(
        "{}",
        format!("最快方式：{} ({}ms)", min_result.0, min_result.2).green()
    );
    println!("{}", "=========================================".green());
    println!();

    // 显示修改清单
    println!("{}", "配置修改清单:".cyan());
    println!("----------------------------------------");

    match min_result.3.as_str() {
        "direct" => {
            println!("  移除镜像配置:");
            println!(
                "    git config --global --unset url.\"{}/\".insteadOf",
                MIRROR_URL
            );
            println!(
                "    git config --global --unset url.\"{}/\".insteadOf",
                MIRROR_URL_BACKUP
            );
            println!("  移除代理配置:");
            println!("    git config --global --unset http.proxy");
            println!("    git config --global --unset https.proxy");
        }
        t if t.starts_with("mirror:") => {
            let mirror_url = &min_result.1;
            println!("  启用镜像：{}", mirror_url);
            println!(
                "    git config --global url.\"{}/\".insteadOf \"https://github.com/\"",
                mirror_url
            );
            println!(
                "    git config --global url.\"{}/\".insteadOf \"git://github.com/\"",
                mirror_url
            );
            println!("  移除代理配置:");
            println!("    git config --global --unset http.proxy");
            println!("    git config --global --unset https.proxy");
        }
        t if t.starts_with("proxy:") => {
            println!("  启用代理：{}", min_result.1);
            println!("    git config --global http.proxy {}", min_result.1);
            println!("    git config --global https.proxy {}", min_result.1);
            println!("  移除镜像配置:");
            println!(
                "    git config --global --unset url.\"{}/\".insteadOf",
                MIRROR_URL
            );
            println!(
                "    git config --global --unset url.\"{}/\".insteadOf",
                MIRROR_URL_BACKUP
            );
        }
        _ => {}
    }

    println!("----------------------------------------");
    println!();
    println!(
        "{}",
        "还原方法：执行选项 0) 恢复默认并退出".yellow()
    );
    println!();

    let confirm = Confirm::new()
        .with_prompt("是否应用此配置？")
        .default(false)
        .interact()
        .unwrap();

    if !confirm {
        println!("已取消");
        return;
    }

    // 先清理所有配置
    let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL)]);
    let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL_BACKUP)]);
    let _ = run_git(&["config", "--global", "--unset", "http.proxy"]);
    let _ = run_git(&["config", "--global", "--unset", "https.proxy"]);

    // 应用最快的配置
    match min_result.3.as_str() {
        "direct" => {
            println!("{}", "已配置为直连模式".blue());
        }
        t if t.starts_with("mirror:") => {
            let mirror_url = &min_result.1;
            let _ = run_git(&[
                "config",
                "--global",
                &format!("url.{}/.insteadOf", mirror_url),
                "https://github.com/",
            ]);
            let _ = run_git(&[
                "config",
                "--global",
                &format!("url.{}/.insteadOf", mirror_url),
                "git://github.com/",
            ]);
            println!(
                "{}",
                format!("✓ 已启用镜像：{}", mirror_url).green()
            );
        }
        t if t.starts_with("proxy:") => {
            let _ = run_git(&["config", "--global", "http.proxy", &min_result.1]);
            let _ = run_git(&["config", "--global", "https.proxy", &min_result.1]);
            println!(
                "{}",
                format!("✓ 已启用代理：{}", min_result.1).green()
            );
        }
        _ => {}
    }

    println!();
    println!("{}", "配置已应用！".green());
}

fn restore_default() {
    println!("{}", "恢复默认设置...".yellow());
    println!();

    // 查找最新的 hosts 备份
    let mut backup_files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir("/etc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("hosts.bak.") {
                backup_files.push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    if !backup_files.is_empty() {
        backup_files.sort();
        if let Some(latest_backup) = backup_files.last() {
            println!("找到备份：{}", latest_backup);
            let confirm = Confirm::new()
                .with_prompt("是否恢复 hosts?")
                .default(false)
                .interact()
                .unwrap();
            if confirm {
                Command::new("sudo")
                    .arg("cp")
                    .arg(latest_backup)
                    .arg("/etc/hosts")
                    .status()
                    .expect("恢复 hosts 失败");
                println!("{}", "✓ hosts 已恢复".green());
            }
        }
    }

    // 移除 Git 镜像配置
    let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL)]);
    let _ = run_git(&["config", "--global", "--unset", &format!("url.{}/.insteadOf", MIRROR_URL_BACKUP)]);

    // 移除 Git 代理
    let _ = run_git(&["config", "--global", "--unset", "http.proxy"]);
    let _ = run_git(&["config", "--global", "--unset", "https.proxy"]);

    println!("{}", "✓ 已恢复默认设置".green());
}