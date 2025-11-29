use glib::{ExitCode, GString};
use gtk4::{AlertDialog, Align, CenterBox, StackTransitionType, prelude::*};
use gtk4::{Application, ApplicationWindow, Label, Box, Orientation, Entry, Button, DropDown, Grid, Stack, StackSwitcher};

pub mod ip4entry;

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

        let sign_in_stack = Stack::builder()
            .transition_type(StackTransitionType::SlideLeftRight)
            .hexpand(true)
            .halign(Align::Center)
            .build();
        let left_sign_in_box = Grid::builder()
            .column_spacing(5)
            .row_spacing(5)
            .halign(Align::Center)
            .build();
        let right_sign_in_box = CenterBox::new();
        {
            {
                let label_a = Label::builder()
                    .label("IPv4:")
                    .justify(gtk4::Justification::Right)
                    .build();
                let label_b = Label::builder()
                    .label("Authenticate Using:")
                    .justify(gtk4::Justification::Right)
                    .build();

                let ip_v4 = ip4entry::IPv4Entry::new();
                let auth_method = DropDown::from_strings(&["Keychain"]);

                
                left_sign_in_box.attach(&label_a, 0, 0, 1, 1);
                left_sign_in_box.attach(&label_b, 0, 1,1, 1);
                left_sign_in_box.attach(&ip_v4, 1, 0, 1, 1);
                left_sign_in_box.attach(&auth_method, 1, 1, 1, 1);
            }

            {
                let frame = Box::builder()
                    .halign(Align::Center)
                    .orientation(Orientation::Horizontal)
                    .spacing(5)
                    .vexpand(false)
                    .build();

                frame.append(&Label::new(Some("IPv4:")));
                let ip_v4 = ip4entry::IPv4Entry::new();
                frame.append(&ip_v4);

                right_sign_in_box.set_center_widget(Some(&frame));
            }

            sign_in_stack.add_titled(&left_sign_in_box, Some("signIn"), "Previous Connection");
            sign_in_stack.add_titled(&right_sign_in_box, Some("newConn"), "New Connection");
        }

        let switcher = StackSwitcher::builder()
            .stack(&sign_in_stack)
            .build();

        let submit = Button::builder()
            .label("Connect")
            .hexpand(false)
            .build();

        let content_box = Box::new(Orientation::Vertical, 5);
        content_box.append(&title_box);
        content_box.append(&switcher);
        content_box.append(&sign_in_stack);
        content_box.append(&submit);

        let window = ApplicationWindow::builder()
            .title("Regis")
            .application(app)
            .child(&content_box)
            .default_height(600)
            .default_width(700)
            .build();

        let internal_error = AlertDialog::builder()
            .modal(true)
            .detail("We are sorry, but an internal error occured.")
            .message("Internal Error")
            .default_button(0)
            .buttons(["Ok"])
            .build();

        submit.connect_clicked(
            glib::clone!(
                #[weak] 
                sign_in_stack,

                #[weak]
                window,

                #[strong]
                internal_error,

                move |_button| {
                    let page = sign_in_stack.visible_child_name();

                    let sign_in = GString::from("signIn");
                    let new_conn = GString::from("newConn");
                    if page == Some(sign_in) {
                        println!("Sign in reached")
                    }
                    else if page == Some(new_conn) {
                        println!("New connection reached")
                    }
                    else {
                        eprintln!("The current page {page:?} could not be parsed as a valid page.");

                        internal_error.show(Some(&window));
                    }
                }
            )
        );

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