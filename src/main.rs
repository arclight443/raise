use anyhow::{Context, Result, bail};
use argh::FromArgs;
use miniserde::{json, Deserialize};
use std::process::{Child, Command};

#[derive(FromArgs)]
/// Raise window if it exists, otherwise launch new window.
struct Args {
    /// class to focus
    #[argh(option, short = 'c')]
    class: String,

    /// command to launch
    #[argh(option, short = 'e')]
    launch: String,

    /// move window to current workspace
    #[argh(switch, short = 'm')]
    move_to_current: bool,

    /// move current window to nearest empty workspace
    #[argh(switch, short = 'n')]
    move_to_nearest_empty: bool,
}

#[derive(Deserialize, Debug)]
struct Client {
    class: String,
    address: String,
}

fn launch_command(args: &Args) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("keyword")
        .arg("exec")
        .arg(&args.launch)
        .spawn()
}

fn focus_window(address: &str) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("focuswindow")
        .arg(format!("address:{address}"))
        .spawn()
}

fn move_to_current(address: &str) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("movetoworkspace")
        .arg(format!("+0,address:{address}"))
        .spawn()
}

fn goto_nearest_empty_workspace() -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("workspace")
        .arg("empty")
        .spawn()
}

fn get_current_matching_window(class: &str) -> Result<Client> {
    let output = Command::new("hyprctl")
        .arg("activewindow")
        .arg("-j")
        .output()?;
    let stdout = String::from_utf8(output.stdout)
        .context("Reading `hyprctl currentwindow -j` to string failed")?;
    let client = json::from_str::<Client>(&stdout)?;
    if class == &client.class {
        Ok(client)
    } else {
        bail!("Current window is not of same class")
    }
}

fn main() -> Result<()> {
    // Get arguments
    let args: Args = argh::from_env();

    if args.move_to_current && args.move_to_nearest_empty {
        eprintln!("Error: --move-to-current and --move-to-nearest-empty cannot be passed at the same time.");
        std::process::exit(1);
    }

    // Launch hyprctl
    let json = Command::new("hyprctl").arg("clients").arg("-j").output();
    match json {
        Ok(output) if output.status.success() => {
            // Deserialize output
            let stdout = String::from_utf8(output.stdout)
                .context("Reading `hyprctl clients -j` to string failed")?;
            let clients = json::from_str::<Vec<Client>>(&stdout)
                .context("Failed to parse `hyprctl clients -j`")?;

            // Filter matching clients
            let candidates = clients
                .iter()
                .filter(|client| client.class == args.class)
                .collect::<Vec<_>>();

            // Are we currently focusing a window of this class?
            if let Ok(Client { address, .. }) = get_current_matching_window(&args.class) {
                // Focus next window based on first
                if let Some(index) = candidates.iter().position(|client| client.address == address) {
                    if let Some(next_client) = candidates.iter().cycle().skip(index + 1).next() {
                        if args.move_to_nearest_empty {
                            goto_nearest_empty_workspace();
                            move_to_current(&next_client.address)?;
                        }
                        else if args.move_to_current { 
                            move_to_current(&next_client.address)?; 
                        }
                        else { focus_window(&next_client.address)?; }
                    }
                }
            } else {
                // Focus first window, otherwise launch command
                match candidates.first() {
                    Some(Client { address, .. }) => {
                        if args.move_to_nearest_empty {
                            goto_nearest_empty_workspace();
                            move_to_current(address)
                        }
                        else if args.move_to_current { 
                            move_to_current(address) 
                        }
                        else { focus_window(address) }
                    },
                    None => { 
                        if args.move_to_nearest_empty { 
                            goto_nearest_empty_workspace();
                            launch_command(&args)
                        }
                        else { launch_command(&args) }
                    },
                };
            }
        }
        // If hyprctl fails, just launch it
        _ => {
            launch_command(&args)?;
        }
    }

    // Success
    Ok(())
}
