use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Select};
use std::time::{SystemTime, UNIX_EPOCH};
use todo_queue_app::{Queue, Todo};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Add { description: String },
    List,
    Done,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let file_path = "todos.bin";

    let mut queue: Queue<Todo> = Queue::load(file_path)?;

    match args.command {
        Commands::Add { description } => {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let todo = Todo {
                id,
                description,
                created_at: id,
            };
            queue.enqueue(todo);
            queue.save(file_path)?;
            println!("Task added successfully!");
        }
        Commands::List => {
            if queue.is_empty() {
                println!("No tasks in the queue.");
            } else {
                for todo in &queue.items {
                    println!("[{}] {}", todo.id, todo.description);
                }
            }
        }
        Commands::Done => {
            if queue.is_empty() {
                println!("No tasks to complete.");
                return Ok(());
            }

            let todos: Vec<String> = queue
                .items
                .iter()
                .map(|todo| todo.description.clone())
                .collect();

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select a task to complete")
                .default(0)
                .items(&todos)
                .interact()
                .unwrap();

            if let Some(todo) = queue.remove(selection) {
                queue.save(file_path)?;
                println!("Completed task: {}", todo.description);
            }
        }
    }

    Ok(())
}
