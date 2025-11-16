use std::net::{AddrParseError, Ipv4Addr};

use gtk4::{Orientation, prelude::*};
use glib::subclass::types::ObjectSubclass;
use glib::subclass::prelude::*;
use gtk4::subclass::box_::BoxImpl;
use gtk4::subclass::widget::WidgetImpl;

mod imp {
    use super::*;
    use gtk4::{Box, Entry};

    #[derive(Default)]
    pub struct IPv4Entry {
        pub entries: [Entry; 4],
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IPv4Entry {
        const NAME: &'static str = "IPv4Entry";
        type Type = super::IPv4Entry;
        type ParentType = Box;
    }

    impl ObjectImpl for IPv4Entry { 
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_spacing(3);
            //obj.set_orientation(Orientation::Horizontal)

            for e in &self.entries {
                e.set_width_chars(3);
                e.set_max_width_chars(3);
                e.set_max_length(3);
                e.set_input_purpose(gtk4::InputPurpose::Digits);
                e.set_vexpand(false);
                e.set_hexpand(false);

                obj.append(e);
            }
        }
    }
    impl WidgetImpl for IPv4Entry { }
    impl BoxImpl for IPv4Entry { }
}

glib::wrapper! {
    pub struct IPv4Entry(ObjectSubclass<imp::IPv4Entry>) 
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl IPv4Entry {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("orientation", Orientation::Horizontal)
            .build()
    }

    pub fn get_ip(&self) -> Result<Ipv4Addr, AddrParseError> {
        let contents: Vec<_> = self.imp().entries.iter().map(|x| x.text().to_string()).collect();
        let total = contents.join(".");

        total.parse()
    }
}