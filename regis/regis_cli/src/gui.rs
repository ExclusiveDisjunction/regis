use std::cell::RefCell;
use std::net::{IpAddr, TcpStream};
use std::net::SocketAddr;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk4::{prelude::*, DialogFlags, Entry, MessageDialog, ScrolledWindow};
use gtk4::{Application, ApplicationWindow, Button, Label, Box as GtkBox, Orientation};

use common::log_error;
use common::lock::{MutexProvider, OptionMutexProvider};
use common::loc::CLIENTS_PORT;
use common::metric::CollectedMetrics;
use common::msg::{decode_response, send_message, RequestMessages, ResponseMessages};
use common::msg::{SendError, DecodeError, send_request};
use common::metric::{Utilization, BinaryNumber, BinaryScale};

use lazy_static::lazy_static;

use crate::tool::SummaryEntry;

pub struct ConnectionProvider {
    data: Arc<Mutex<Option<TcpStream>>>
}
impl MutexProvider for ConnectionProvider {
    type Data = Option<TcpStream>;
    fn access_raw(&self) -> common::lock::ProtectedAccess<'_, std::sync::Arc<std::sync::Mutex<Self::Data>>> {
        common::lock::ProtectedAccess::new(&self.data)
    }
}
impl OptionMutexProvider<TcpStream> for ConnectionProvider { }
impl Default for ConnectionProvider {
    fn default() -> Self {
        Self {
            data: Arc::new(Mutex::new(None))
        }
    }
}
impl ConnectionProvider {
    pub fn try_open(&self, addr: IpAddr, port: u16) -> Result<(), std::io::Error> {
        let listener = TcpStream::connect(SocketAddr::from((addr, port)))?;

        self.pass(listener);
        Ok(())
    }
}

lazy_static! {
    pub static ref COMM: ConnectionProvider = ConnectionProvider::default();
}

pub fn gui_entry() -> Result<(), ExitCode> {
    let app = Application::new(Some("com.exdisj.regis"), Default::default());

    app.connect_activate(build_ui);

    app.run();

    Ok(())
}

fn build_ui(app: &Application) {
    let window = Rc::new(
        ApplicationWindow::builder()
        .application(app)
        .title("Regis")
        .default_width(600)
        .default_height(700)
        .build()
    );

    
    build_connect_window(Rc::clone(&window));

    window.show();
}

fn build_connect_window(win: Rc<ApplicationWindow>) {
    win.set_title(Some("Regis (Disconnected)"));

    let vbox = GtkBox::new(Orientation::Vertical, 10);

    let ip: Rc<RefCell<Option<IpAddr>>> = Rc::new(RefCell::new(None));

    let title = Label::new(Some("New Connection"));
    let ip_label = Label::new(Some("IP:"));
    let ip_entry = Entry::new();
    let connect_btn = Button::new();

    connect_btn.set_label("Connect");
    let cloned_ip = Rc::clone(&ip);
    let connect_win = Rc::clone(&win);
    connect_btn.connect_clicked(move |_| {
        let ip = match cloned_ip.borrow_mut().take() {
            Some(v) => v,
            None => {
                let dialog = MessageDialog::new(
                    Some(&*connect_win),
                    DialogFlags::MODAL,
                    gtk4::MessageType::Error,
                    gtk4::ButtonsType::Ok,
                    "Please provide a valid IP address to connect to."
                );

                dialog.set_title(Some("Error: IP"));
                dialog.connect_response(|dialog, _| dialog.close());

                dialog.show();
                return;
            }
        };

        if let Err(e) = COMM.try_open(ip, CLIENTS_PORT) {
            log_error!("Unable to open '{e}'");

            let dialog = MessageDialog::new(
                Some(&*connect_win),
                DialogFlags::MODAL,
                gtk4::MessageType::Error,
                gtk4::ButtonsType::Ok,
                "The connection could not be made. Ensure that the IP is correct, and that regisd is running."
            );
            // PUt the value back into the placeholder

            *cloned_ip.borrow_mut() = Some(ip);

            dialog.set_title(Some("Error: Connection Failed"));
            dialog.connect_response(|dialog, _| dialog.close());

            dialog.show();
            return;
        }

        //Connection has been made, go to the main screen.
        let its_handle = Rc::clone(&connect_win);
        build_main_window(its_handle);
    });

    let entry_ip = Rc::clone(&ip);
    ip_entry.connect_changed(move |x| {
        let text = x.text().to_string();
        match text.parse() {
            Ok(v) => *entry_ip.borrow_mut() = Some(v),
            Err(e) => {
                eprintln!("Unable to parse IP as a valid IP literal '{e}'");
            }
        }
    });

    vbox.append(&title);
    vbox.append(&ip_label);
    vbox.append(&ip_entry);
    vbox.append(&connect_btn);

    vbox.set_halign(gtk4::Align::Fill);
    vbox.set_valign(gtk4::Align::Fill);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    win.set_child(Some(&vbox));
}

pub const CHART_X_AMOUNT: usize = 10;

fn build_main_window(win: Rc<ApplicationWindow>) {
    let scroll_window = ScrolledWindow::new();
    scroll_window.set_hexpand(true);
    scroll_window.set_vexpand(true);
    let host_box = GtkBox::new(Orientation::Vertical, 10);
    host_box.set_halign(gtk4::Align::Fill);
    host_box.set_valign(gtk4::Align::Fill);

    let window_title = Label::new(Some("Regis"));
    win.set_title(Some("Regis (Connected)"));
    win.set_width_request(1100);
    host_box.append(&window_title);

    let refresh_button = Button::new();
    refresh_button.set_label("Refresh");
    host_box.append(&refresh_button);

    {
        let metrics_box = GtkBox::new(Orientation::Vertical, 10);
        let title = Label::new(Some("Metrics (Ordered from least recent (left) to most recent (right)): "));

        let labels: Rc<RefCell<Vec<Label>>> = Rc::new(RefCell::new(vec![]));
        let grid: Rc<gtk4::Grid> = Rc::new(gtk4::Grid::new());

        let header_column = vec![Label::new(Some("Time")), Label::new(Some("CPU")), Label::new(Some("Memory")), Label::new(Some("Process Count"))];
        for (i, header) in header_column.into_iter().enumerate() {
            grid.attach(&header, 0, i as i32, 1, 1);
        }

        let update_labels = Rc::clone(&labels);
        let update_grid = Rc::clone(&grid);
        let update_func = move || {
            let mut lock = COMM.access();
            let access = match lock.access_mut() {
                Some(v) => v,
                None => {
                    log_error!("Unable to get the stream.");
                    return;
                }
            };
            let data = request_fresh(access, CHART_X_AMOUNT).expect("Idk man");
            let summary = summarize(data);

            for row in update_labels.borrow().iter() {
                update_grid.remove(row);
                row.set_visible(false);
                
            }
            labels.borrow_mut().clear();
        
            let mut labels_mut = update_labels.borrow_mut();

            for (col, item) in summary.into_iter().enumerate() {
                let time = chrono::DateTime::from_timestamp(item.time,0).expect("Unable to get the time.").time();
                
                let text = vec![time.to_string(), item.cpu_usage.to_string(), item.mem_usage.to_string(), item.proc_count.to_string()];
                let my_labels: Vec<Label> = text.into_iter().map(|x| Label::new(Some(&x))).collect();
        
                for (row, label) in my_labels.into_iter().enumerate() {
                    label.set_margin_end(2);
                    update_grid.attach(&label, col as i32 + 1, row as i32, 1, 1);
                    labels_mut.push(label);
                }
            }
        };

        refresh_button.connect_clicked(move |_| {
            update_func();
        });

        metrics_box.append(&title);
        metrics_box.append(&*grid);

        host_box.append(&metrics_box);


    }

    {
        let curr_stat_box = GtkBox::new(Orientation::Vertical, 10);
        let title = Label::new(Some("Current Information:"));

        let status_text = Rc::new(Label::new(Some("(No information)")));
        status_text.set_wrap(true);
        status_text.set_wrap_mode(gtk4::pango::WrapMode::Word);

        let refresh_status_text = Rc::clone(&status_text);
        refresh_button.connect_clicked(move |_| {
            let mut lock = COMM.access();
            let access = match lock.access_mut() {
                Some(v) => v,
                None => {
                    log_error!("Unable to get the stream.");
                    return;
                }
            };

            let request = RequestMessages::Status;
            if let Err(e) = send_message(request, access) {
                log_error!("Unable to send message '{e}'");
                return;
            }

            let response: ResponseMessages = match decode_response(access) {
                Ok(v) => v,
                Err(e) => {
                    log_error!("Unable to decode message '{e}'");
                    return;
                }
            };

            let data = match response {
                ResponseMessages::Status(s) => s,
                _ => {
                    log_error!("Got something I didnt expect, wanted status.");
                    return;
                }
            };

            let text = data.to_string();
            refresh_status_text.set_label(&text);
        });

        curr_stat_box.append(&title);
        curr_stat_box.append(&*status_text);

        host_box.append(&curr_stat_box);
    }

    host_box.set_margin_bottom(10);
    host_box.set_margin_top(10);
    host_box.set_margin_start(10);
    host_box.set_margin_end(10);
    scroll_window.set_child(Some(&host_box));
    win.set_child(Some(&scroll_window));
}

pub fn request_fresh(connection: &mut TcpStream, amount: usize) -> Result<Vec<CollectedMetrics>, std::io::Error> {
    let request = RequestMessages::Metrics(amount);
    if let Err(e) = send_request(request, connection) {
        match e {
            SendError::IO(io) => return Err(io),
            SendError::Serde(s) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, s))
        }
    }

    let response: ResponseMessages = match decode_response(connection) {
        Ok(v) => v,
        Err(e) => {
            match e {
                DecodeError::IO(io) => return Err(io),
                DecodeError::Serde(serde) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, serde)),
                DecodeError::UTF(utf) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, utf))
            }
        }
    };

    let extracted = match response {
        ResponseMessages::Metrics(v) => v.info,
        _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "should not have gotten anything but the metrics response."))
    };

    Ok(extracted)
}

pub fn summarize(mut data: Vec<CollectedMetrics>) -> Vec<SummaryEntry> {
    data.sort_by(|x, y| x.time.cmp(&y.time));

    let mut result = vec![];
    for metric in data {
        let time = metric.time;

        let cpu: Utilization = Utilization::new(
            metric.cpu.as_ref()
            .map(|x| x.nice.inner + x.user.inner + x.system.inner)
            .unwrap_or(0))
            .unwrap_or(Utilization::new_unwrap(0)
        );
        let ram = metric.memory.as_ref()
            .map(|x| x.metrics.iter()
                .find(|x| x.name == "Mem"))
            .flatten()
            .map(|x| x.available)
            .flatten()
            .unwrap_or(BinaryNumber::new(0.0, BinaryScale::Byte));
        let proc_count = metric.proc_count.as_ref()
            .map(|x| x.count)
            .unwrap_or(0);

        let summary = SummaryEntry { time, cpu_usage: cpu, mem_usage: ram, proc_count };

        result.push(summary);
    }

    result
}