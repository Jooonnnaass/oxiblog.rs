use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};

use anyhow::{anyhow, bail, Ok};
use clap::Parser;
use pulldown_cmark::{html, Options};
use serde::Deserialize;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    blog_dir: String,

    #[arg(short, long)]
    projects_dir: String,

    #[arg(short, long)]
    images_dir: String,

    #[arg(short, long)]
    output_dir: String,
}

#[derive(Deserialize, Debug)]
struct Frontmatter {
    title: String,
    tags: Vec<String>,
    release_date: String,
    summary: String,
    image: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let current_dir = env::current_dir()?;
    let output_dir = Path::join(&current_dir, args.output_dir);
    let image_output_dir = Path::join(&output_dir, "images/");
    let image_dir = Path::join(&current_dir, args.images_dir);
    let blog_dir = Path::join(&current_dir, args.blog_dir);
    let project_dir = Path::join(&current_dir, args.projects_dir);
    let mut sql_output = String::new();

    fs::create_dir_all(&image_output_dir)?;

    for entry in WalkDir::new(&image_dir)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| {
            e.path().extension().map_or(false, |ext| {
                matches!(ext.to_str(), Some("png" | "jpg" | "jpeg" | "gif" | "svg"))
            })
        })
    {
        fs::copy(
            entry.path(),
            Path::join(&image_output_dir, entry.file_name()),
        )?;
    }

    for entry in WalkDir::new(&blog_dir)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    {
        sql_output.push_str(&parse_markdown(entry.path(), "blog_posts")?);
    }

    for entry in WalkDir::new(&project_dir)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    {
        sql_output.push_str(&parse_markdown(entry.path(), "project_posts")?);
    }

    File::create(Path::join(&output_dir, "migration.sql"))?.write_all(sql_output.as_bytes())?;

    Ok(())
}

fn parse_markdown(path: &Path, table: &str) -> anyhow::Result<String> {
    let content = fs::read_to_string(path)?;
    let parts: Vec<&str> = content.splitn(3, "---").collect();

    if parts.len() < 3 {
        bail!("Invalid Markdown file format");
    }

    let frontmatter: Frontmatter = serde_yaml::from_str(&parts[1])?;

    let parser = pulldown_cmark::Parser::new_ext(parts[2], Options::all());
    let mut html = String::new();
    html::push_html(&mut html, parser);

    let sql = format!(
        "INSERT INTO {} (slug, title, tags, release_date, summary, image, html) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}');\n",
        table,
        frontmatter.title.to_lowercase().replace(" ", "-").replace("'", "''"),
        frontmatter.title.replace("'", "''"),
        frontmatter.tags.join(",").replace("'", "''"),
        frontmatter.release_date,
        frontmatter.summary.replace("'", "''"),
        frontmatter.image.replace("'", "''"),
        html.replace("'", "''"),
    );

    Ok(sql)
}
