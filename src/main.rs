use octocrab::{models::pulls::PullRequest, Octocrab};
use std::env;
use std::fs;
use std::path::Path;
use syn::{visit::Visit, File, Item};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 環境変数からGitHubトークンを取得
    let token = env::var("GITHUB_TOKEN")
        .expect("GITHUB_TOKEN を環境変数に設定してください");

    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("使い方: review_tool --owner OWNER --repo REPO --pr PR_NUMBER");
        std::process::exit(1);
    }

    let owner = &args[1];
    let repo = &args[2];
    let pr_number: u64 = args[3].parse()?;

    let octo = Octocrab::builder().personal_token(token).build()?;
    let pr: PullRequest = octo
        .pulls(owner, repo)
        .get(pr_number)
        .await?;

    println!("# PR {}: {}", pr.number.unwrap_or(0), pr.title.unwrap_or_default());
    println!("変更ファイル:");

    let files = octo
        .pulls(owner, repo)
        .list_files(pr_number)
        .await?
        .take_items();

    for file in files.iter().filter(|f| f.filename.ends_with(".rs")) {
        println!("- {}", file.filename);

        let local_path = Path::new(&file.filename);
        if local_path.exists() {
            let content = fs::read_to_string(local_path)?;
            analyze_rust_file(&file.filename, &content);
        } else {
            println!("  ※ ローカルにファイルがありません: {}", file.filename);
        }
    }

    Ok(())
}

fn analyze_rust_file(filename: &str, source: &str) {
    let syntax: File = syn::parse_file(source).unwrap_or_else(|_| {
        panic!("構文解析に失敗しました: {}", filename);
    });

    let mut visitor = FunctionCollector::default();
    visitor.visit_file(&syntax);

    println!("## 関数一覧（{}）", filename);
    for f in visitor.functions {
        println!("- {}", f);
    }
}

#[derive(Default)]
struct FunctionCollector {
    functions: Vec<String>,
}

impl<'ast> Visit<'ast> for FunctionCollector {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.functions.push(i.sig.ident.to_string());
    }

    fn visit_item(&mut self, i: &'ast Item) {
        if let Item::Fn(func) = i {
            self.visit_item_fn(func);
        }
    }
}
