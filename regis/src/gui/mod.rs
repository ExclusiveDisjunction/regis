use glib::ExitCode;
use gtk4::ffi::GtkStringList;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Label, Box, Orientation, Entry, Button, DropDown, Grid};

pub const APPLICATION_ID: &'static str = "com.exdisj.regis.regis";

pub fn gui_entry() -> Result<(), ExitCode> {
    let app = Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    app.connect_activate(|app| {
        let title_box = Box::new(Orientation::Vertical, 5);
        {
            let title = Label::builder()
                .label("Welcome to Regis")
                .build();
            let sub_title = Label::new(Some("Please connect to a server"));

            title_box.append(&title);
            title_box.append(&sub_title);
        }

        let sign_in_box = Box::new(Orientation::Horizontal, 5);
        {
            let left_sign_in_box = Box::new(Orientation::Vertical, 5);
            {
                let title = Label::new(Some("Previous Connection"));

                let label_a = Label::builder()
                    .label("IPv4:")
                    .justify(gtk4::Justification::Right)
                    .build();
                let label_b = Label::builder()
                    .label("Authenticate Using:")
                    .justify(gtk4::Justification::Right)
                    .build();

                let ip_v4 = Entry::new();
                let auth_method = DropDown::from_strings(&["Keychain"]);

                let grid = Grid::builder()
                    .column_spacing(5)
                    .row_spacing(5)
                    .build();
                grid.attach(&label_a, 0, 0, 1, 1);
                grid.attach(&label_b, 0, 1,1, 1);
                grid.attach(&ip_v4, 1, 0, 1, 1);
                grid.attach(&auth_method, 1, 1, 1, 1);

                let submit = Button::builder()
                    .label("Connect")
                    .build();

                submit.connect_clicked(|x| {
                    todo!("Handle connection...");
                });

                left_sign_in_box.append(&title);
                left_sign_in_box.append(&grid);
                left_sign_in_box.append(&submit);
            }

            let right_sign_in_box = Box::new(Orientation::Vertical, 5);
            {
                let title = Label::new(Some("New Connection"));

                let container = Box::new(Orientation::Horizontal, 5);
                container.append(&Label::new(Some("IPv4:")));
                let ip_v4 = Entry::new();
                container.append(&ip_v4);

                let submit = Button::builder()
                    .label("Connect")
                    .build();

                submit.connect_clicked(|x| {
                    todo!("Handle connection...");
                });

                right_sign_in_box.append(&title);
                right_sign_in_box.append(&container);
                right_sign_in_box.append(&submit);
            }

            let divider = gtk4::Separator::new(Orientation::Vertical);

            sign_in_box.append(&left_sign_in_box);
            sign_in_box.append(&divider);
            sign_in_box.append(&right_sign_in_box);
        }

        let content_box = Box::new(Orientation::Vertical, 5);
        content_box.append(&title_box);
        content_box.append(&sign_in_box);

        let window = ApplicationWindow::builder()
            .title("Regis")
            .application(app)
            .child(&content_box)
            .default_height(600)
            .default_width(700)
            .build();

        window.present()
    });

    let exit = app.run_with_args::<&str>(&[]);
    if exit == ExitCode::SUCCESS {
        Ok( () )
    }
    else {
        Err( exit )
    }
}