
use gio::ListStore;
use gtk::glib;
use gtk::subclass::{prelude::*};
use std::cell::RefCell;
use gtk::{ColumnView, prelude::*, SingleSelection, Entry, ListItem, FlowBox};
use crate::widgets::split_panel::splits::VideoSegment;
use crate::widgets::split_panel::timeentry::TimeEntry;
use crate::helpers::data::{store_data, get_data};
use crate::helpers::format::format_clock;
use crate::helpers::parse::{string_to_nseconds, validate_split_table_entry};
use crate::widgets::video_player_widget::video_player::VideoPlayer;


mod imp {

    use super::*;

    #[derive(Default)]
    pub struct SplitTable {
        pub split_table_column_view: RefCell<Option<ColumnView>>,
        pub split_table_liststore: RefCell<Option<ListStore>>,
        pub start_time_offset_column_view: RefCell<Option<ColumnView>>,
        pub start_time_offset_liststore: RefCell<Option<ListStore>>,
        pub max_number_of_video_players: RefCell<u32>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for SplitTable {
        const NAME: &'static str = "SplitTable";
        type Type = super::SplitTable;
    }

    impl SplitTable {
        fn create_column_view<T: 'static + IsA<glib::Object>>(&self) -> (ListStore, ColumnView) {
            let model = gio::ListStore::new::<T>();
            let model_clone = model.clone();

            let selection_model = gtk::SingleSelection::new(Some(model_clone));
            
            // Create the ColumnView
            let column_view = gtk::ColumnView::new(Some(selection_model));
            
            (model, column_view)
        }
    }

    impl ObjectImpl for SplitTable {
        fn constructed(&self) {
            let (split_table_liststore, split_table_column_view) = self.create_column_view::<VideoSegment>();
            split_table_column_view.set_reorderable(false);
            split_table_column_view.set_show_column_separators(true);
            split_table_column_view.set_show_row_separators(true);
            split_table_column_view.add_css_class("data-table");
            let (start_time_offset_liststore, start_time_offset_column_view) = self.create_column_view::<TimeEntry>();
            self.split_table_column_view.borrow_mut().replace(split_table_column_view);
            self.split_table_liststore.borrow_mut().replace(split_table_liststore);
            self.start_time_offset_column_view.borrow_mut().replace(start_time_offset_column_view);
            self.start_time_offset_liststore.borrow_mut().replace(start_time_offset_liststore);
        }
    }
}

glib::wrapper! {
    pub struct SplitTable(ObjectSubclass<imp::SplitTable>)
    @implements gtk::Buildable;
}

impl SplitTable {
    pub fn new() -> Self {
        let split_table: Self = glib::Object::new::<Self>();
        let imp = imp::SplitTable::from_obj(&split_table);
        *imp.max_number_of_video_players.borrow_mut() = 1;
        split_table
    }

    pub fn set_split(&self, video_player_index: u32, video_player_position: u64) -> Result<(), String> {
        let imp = self.imp();
        let split_table_column_view_borrow = imp.split_table_column_view.borrow();
        let split_table_column_view = match split_table_column_view_borrow.as_ref() {
            Some(ls) => ls,
            None => return Err("Missing split_table_column_view".to_string()),
        };
        let selection_model = split_table_column_view.model().and_downcast::<SingleSelection>().unwrap();
        if let Some(selected_segment) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            let selected_index = selection_model.selected();
            selected_segment.set_time(video_player_index as usize, video_player_position);
            self.correct_conflicts(video_player_index, selected_index);
            Ok(())
        } else {
            Err("Segment not selected".to_string())
        }
    }

    pub fn set_start_time_offset(&self, video_player_index: u32, video_player_position: u64) -> Result<(), String> {
        let imp = self.imp();
        let start_time_offset_liststore_borrow = imp.start_time_offset_liststore.borrow();
        let start_time_offset_liststore = match start_time_offset_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return Err("Missing start_time_offset_liststore".to_string()),
        };
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore = match split_table_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return Err("Missing split_table_liststore".to_string()),
        };

        let start_offset_time_entry = start_time_offset_liststore.item(video_player_index).and_downcast::<TimeEntry>().unwrap();
        start_offset_time_entry.set_time(video_player_position);

        for i in 0..split_table_liststore.n_items() {
            let video_segment = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            if i == 0 {
                let time = video_segment.get_time(video_player_index as usize).unwrap();
                video_segment.set_duration(video_player_index as usize, time);
            }
            video_segment.set_offset(video_player_index as usize, video_player_position);
        }
        Ok(())
    }

    pub fn add_start_time_offset_row(&self) -> Result<TimeEntry, String> {
        let imp = self.imp();
        let start_time_offset_liststore_borrow = imp.start_time_offset_liststore.borrow();
        let start_time_offset_liststore = match start_time_offset_liststore_borrow.as_ref() {
            Some(stol) => stol,
            None => return Err("Missing start_time_offset_liststore".to_string()),
        };

        let new_start_time_offset_time_entry = TimeEntry::new(0);
        start_time_offset_liststore.append(&new_start_time_offset_time_entry);
        Ok(new_start_time_offset_time_entry)
    }

    pub fn append_empty_row(&self) {
        let imp = self.imp();
        let liststore_borrow = imp.split_table_liststore.borrow();
        let liststore = match liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };

        let insert_index = liststore.n_items();
        self.insert_empty_row(insert_index);
    }

    pub fn insert_empty_row(&self, insert_index: u32) {
        let imp = self.imp();
        let liststore_borrow = imp.split_table_liststore.borrow();
        let liststore = match liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };        
        let new_row_segment = VideoSegment::new("Segment Name");
        for _ in 0..*imp.max_number_of_video_players.borrow() {
            new_row_segment.add_empty_segment();
        }
        liststore.insert(insert_index, &new_row_segment);
    }

    pub fn update_durations(&self, video_player_index: u32, starting_row_index: u32) {
        let imp = self.imp();
        let liststore_borrow = imp.split_table_liststore.borrow();
        let liststore = match liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };
        
        let number_of_rows = liststore.n_items();
        if number_of_rows == 0 {
            return;
        }
        let mut previous_time: u64 = match self.get_previous_time(video_player_index, starting_row_index) {
            Some(time) => time,
            None => {
                let video_segment = liststore.item(0).and_downcast::<VideoSegment>().unwrap();
                video_segment.get_offset(video_player_index as usize)
            },
        };
        for i in starting_row_index..number_of_rows {
            let current_video_segment = liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let current_time = current_video_segment.get_time(video_player_index as usize);
            let current_duration = current_video_segment.get_duration(video_player_index as usize);
            if let (Some(time), Some(_duration)) = (current_time, current_duration) {
                if time == u64::MAX {
                    continue;
                }
                current_video_segment.set_duration(video_player_index as usize, time - previous_time);
                previous_time = time;
            }
        }
    }

    pub fn get_previous_time(&self, video_player_index: u32, row_index: u32) -> Option<u64> {
        let imp = self.imp();
        let liststore_borrow = imp.split_table_liststore.borrow();
        let liststore = match liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return None,
        }; 
    
    for i in (0..row_index).rev() {
        let item = liststore.item(i).and_downcast::<VideoSegment>().unwrap();
        if let Some(time) = item.get_time(video_player_index as usize) {
            if time != u64::MAX {
                return Some(time);
            }
        }
    }
    return None;
}

    pub fn correct_conflicts(&self, video_player_index: u32, starting_row_index: u32) {
        let imp = self.imp();
        let liststore_borrow = imp.split_table_liststore.borrow();
        let liststore = match liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };

        let starting_row = liststore.item(starting_row_index).and_downcast::<VideoSegment>().unwrap();
        let starting_row_time = starting_row.get_time(video_player_index as usize).unwrap();
        for i in (0..starting_row_index).rev() {
            let current_row = liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            match current_row.get_time(video_player_index as usize) {
                Some(time) => {
                    if time == u64::MAX {
                        continue;
                    }
                    if time > starting_row_time {
                        current_row.set_time(video_player_index as usize, starting_row_time);
                    } else {
                        break;
                    }
                }
                None => { }
            }
        }
        for i in starting_row_index+1..liststore.n_items() {
            let current_row = liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            match current_row.get_time(video_player_index as usize) {
                Some(time) => {
                    if time == u64::MAX {
                        continue;
                    }
                    if time < starting_row_time {
                        current_row.set_time(video_player_index as usize, starting_row_time);
                    } else {
                        break;
                    }
                }
                None => { }
            }
        }
        self.update_durations(video_player_index, 0);
    }

    pub fn add_column(&self, title: &str, video_player_index: u32, property_name: &str) {
        let imp = self.imp();
        let liststore_borrow = imp.split_table_liststore.borrow();
        let liststore = match liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        }; 
        let column_view_borrow = imp.split_table_column_view.borrow();
        let column_view = match column_view_borrow.as_ref() {
            Some(cv) => cv,
            None => return,
        }; 

        
        
        let factory = gtk::SignalListItemFactory::new();
        let liststore_clone = liststore.clone();
        let property = property_name.to_string();
        let column_view_clone = column_view.clone();
        // Creates the entry objects
        factory.connect_setup(glib::clone!(
            #[weak(rename_to = this)] self,
            move |_, list_item| {
            let entry = gtk::Entry::new();
            entry.add_css_class("flat");
            entry.set_hexpand(true);
            entry.set_halign(gtk::Align::Fill);
            // When user enters new time into an entry the corresponding values in the videosegment will be updated
            // Any affected values are also updated
            entry.connect_activate(glib::clone!(
                #[strong] property,
                #[weak(rename_to = entry)] entry,
                #[weak(rename_to = list_item)] list_item,
                #[weak(rename_to = liststore)] liststore_clone,
                move |_| {
                    if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                        match &property {
                            prop if prop.starts_with("name") => {
                                // do name stuff here
                                println!("Change name not impletemented");
                            }
                            prop if prop.starts_with("relative-time-") => {
                                println!("Changing {}", property);
                                let row_index = liststore.find(&video_segment).unwrap();
                                let valid_entry = validate_split_table_entry(&entry);
                                if !valid_entry { // Restores segment data if invalid entry
                                    let stored_entry_data = video_segment.property(property.as_str());
                                    entry.set_text(format_clock(stored_entry_data).as_str());
                                } else { // updates segment data with new entry and fixes any conflicts
                                    let new_time = string_to_nseconds(&entry.text().to_string()).unwrap();
                                    video_segment.set_time(video_player_index as usize, new_time);
                                    this.correct_conflicts(video_player_index, row_index);
                                }
                            }
                            prop if prop.starts_with("duration-") => {
                                println!("Changing {}", property);
                                let row_index = liststore.find(&video_segment).unwrap();
                                let valid_entry = validate_split_table_entry(&entry);
                                if !valid_entry {// Restores segment data if invalid entry
                                    let stored_entry_data = video_segment.property(property.as_str());
                                    entry.set_text(format_clock(stored_entry_data).as_str());
                                } else { // updates segment data with new entry and fixes any conflicts
                                    let new_duration = string_to_nseconds(&entry.text().to_string()).unwrap();
                                    video_segment.set_duration(video_player_index as usize, new_duration);
                                    let previous_time: u64 = match this.get_previous_time(video_player_index, row_index) {
                                        Some(time) => time,
                                        None => 0,
                                    };
                                    video_segment.set_time(video_player_index as usize, previous_time + new_duration);
                                    this.correct_conflicts(video_player_index, row_index);
                                }
                            }
                            _ => {
                                eprintln!("Invalid property: {}", property);
                            }
                        }
                    }
                }
            ));
        
            entry.connect_has_focus_notify(glib::clone!(
                #[weak(rename_to = liststore)] liststore_clone,
                #[weak(rename_to = column_view)] column_view_clone,
                #[weak(rename_to = list_item)] list_item,
                move |_| {
                    if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                        let row_index = liststore.find(&video_segment).unwrap_or(u32::MAX);
                        if row_index != u32::MAX {
                            let selection_model = column_view.model().and_downcast::<SingleSelection>().unwrap();
                            selection_model.select_item(row_index, true);
                        }
                    }
                }
            ));
            list_item.set_child(Some(&entry));
        }));

        // Binds the stored data to the displayed entry objects
        let property = property_name.to_string();
        factory.connect_bind(move |_, list_item| {
            let item = list_item.item().and_then(|obj| obj.downcast::<VideoSegment>().ok()).expect("The item is not a VideoSegment");
            let entry = list_item.child().and_then(|child| child.downcast::<Entry>().ok()).expect("The child widget is not Entry");
            // Binds the u64 stored in the video segment to the entries formatted clock
            // Any changes to the videosegment will be updated in the entry object
            item.bind_property(&property, &entry, "text")
                .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
                .transform_to(move |_, value: u64| { 
                    Some(format_clock(value).to_value())
                })
                .build();
        });

        let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
        column_view.append_column(&column);
    }

    pub fn add_name_column(&self, title: &str) {
        let imp = self.imp();
        let column_view_borrow = imp.split_table_column_view.borrow();
        let column_view = match column_view_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };
        let liststore_borrow = imp.split_table_liststore.borrow();
        let liststore = match liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        }; 


        let factory = gtk::SignalListItemFactory::new();
        let column_view_clone = column_view.clone();
        let liststore_clone = liststore.clone();
        // Creates the entry objects
        factory.connect_setup(move |_factory, list_item: &ListItem| {
            let entry = Entry::new();
            entry.add_css_class("flat");
            entry.set_hexpand(true);
            entry.set_halign(gtk::Align::Fill);
            
            // Updates segment name from user input
            entry.connect_activate(glib::clone!(
                #[weak(rename_to = list_item)] list_item,
                move |entry| {
                    if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                        let new_name = entry.text().to_string();
                        video_segment.set_name(new_name);
                    }
                } 
            ));
        
            entry.connect_has_focus_notify(glib::clone!(
                #[weak(rename_to = column_view)] column_view_clone,
                #[weak(rename_to = list_item)] list_item,
                #[weak(rename_to = liststore)] liststore_clone,
                move |_| {
                    if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                        println!("Changed focus");
                        let selection_model = column_view.model().and_downcast::<SingleSelection>().unwrap();
                        let row_index = liststore.find(&video_segment).unwrap_or(u32::MAX);
                        if row_index != u32::MAX {
                            selection_model.select_item(row_index, true);
                        }
                    }
                }
            ));
            list_item.set_child(Some(&entry));
        });
        
        // Binds the stored data to the displayed entry objects
        factory.connect_bind(move |_factory, list_item: &ListItem| {
            let entry = list_item.child().unwrap().downcast::<Entry>().expect("The child is not an Entry");
            let item = list_item.item();
            let video_segment = item.and_downcast_ref::<VideoSegment>().expect("Item is not a VideoSegment");
            let current_name = video_segment.get_name();
            entry.set_text(&current_name);
        });

        
        // Adds the new column to column view
        let new_column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
        column_view.append_column(&new_column);
    }

    pub fn set_max_number_of_video_players(&self, max_number_of_video_players: u32) {
        let imp = self.imp();
        *imp.max_number_of_video_players.borrow_mut() = max_number_of_video_players;
    }

    pub fn setup_start_time_offset_column(&self, title: &str) {
        let imp = self.imp();
        let start_time_offset_liststore_borrow = imp.start_time_offset_liststore.borrow();
        let start_time_offset_liststore = match start_time_offset_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore = match split_table_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };
        let start_time_offset_column_view_borrow = imp.start_time_offset_column_view.borrow();
        let start_time_offset_column_view = match start_time_offset_column_view_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };
        
        
        
        let factory = gtk::SignalListItemFactory::new();
        let start_time_offset_liststore_clone = start_time_offset_liststore.clone();
        let split_table_liststore_clone = split_table_liststore.clone();
        // Creates the entry objects
        factory.connect_setup(move |_, list_item| {
            let entry = gtk::Entry::new();
            entry.add_css_class("flat");
            entry.set_hexpand(true);
            entry.set_halign(gtk::Align::Fill);
            
            entry.connect_activate(glib::clone!(
                #[weak(rename_to = start_time_offset_liststore)] start_time_offset_liststore_clone,
                #[weak(rename_to = list_item)] list_item,
                #[weak(rename_to = entry)] entry,
                #[weak(rename_to = split_table_liststore)] split_table_liststore_clone,
                move |_| {
                    if let Some(time_entry) = list_item.item().and_downcast::<TimeEntry>() {
                        let valid_entry = validate_split_table_entry(&entry);
                        if !valid_entry {
                            let time_entry_data = time_entry.get_time();
                            entry.set_text(format_clock(time_entry_data).as_str());
                        } else {
                            let new_time = string_to_nseconds(&entry.text().to_string()).unwrap();
                            time_entry.set_time(new_time);
                            let video_player_index = start_time_offset_liststore.find(&time_entry).unwrap();
                            for i in 0..split_table_liststore.n_items() {
                                let video_segment = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
                                video_segment.set_offset(video_player_index as usize, new_time);
                                
                            }
                            //update_times(&split_table_model, video_player_index, 0);
                        }
                    }
                }
            ));
            list_item.set_child(Some(&entry));
        });
        
        // Binds the stored data to the displayed entry objects
        factory.connect_bind(move |_, list_item| {
            let item = list_item.item().and_then(|obj| obj.downcast::<TimeEntry>().ok()).expect("The item is not a VideoSegment");
            let entry = list_item.child().and_downcast::<Entry>().expect("The child widget is not entry");
            // Binds the value in the time entry to the entry text field
            // Any changes to the time entries value will be updated in the entry object
            item.bind_property("time", &entry, "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .transform_to(|_, value: u64| { // Converts the u64 time to formatted MM:SS.sss time to display
                Some(format_clock(value).to_value())
            })
            .build();
        
        });
    
        let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
        start_time_offset_column_view.append_column(&column);
    }
    
    pub fn get_split_table_column_view(&self) -> Option<ColumnView> {
        let imp = self.imp();
        let split_table_column_view_borrow = imp.split_table_column_view.borrow();
        let split_table_column_view = match split_table_column_view_borrow.as_ref() {
            Some(cv) => cv,
            None => return None,
        }; 
        Some(split_table_column_view.clone())
    }

    pub fn get_start_time_offset_column_view(&self) -> Option<ColumnView> {
        let imp = self.imp();
        let start_time_offset_column_view_borrow = imp.start_time_offset_column_view.borrow();
        let start_time_offset_column_view = match start_time_offset_column_view_borrow.as_ref() {
            Some(cv) => cv,
            None => return None,
        }; 
        Some(start_time_offset_column_view.clone())
    }

    pub fn get_split_table_liststore(&self) -> Option<ListStore> {
        let imp = self.imp();
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore = match split_table_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return None,
        };
        Some(split_table_liststore.clone())
    }

    pub fn get_start_time_offset_liststore(&self) -> Option<ListStore> {
        let imp = self.imp();
        let start_time_offset_liststore_borrow = imp.start_time_offset_liststore.borrow();
        let start_time_offset_liststore = match start_time_offset_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return None,
        };
        Some(start_time_offset_liststore.clone())
    }

    pub fn connect_row_to_seekbar(&self, video_player_container: &FlowBox, row_index: u32) {
        let imp = self.imp();
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore = match split_table_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };

        let video_player_count = *unsafe { get_data::<usize>(video_player_container, "count").unwrap().as_ref() } as i32;
        let row = split_table_liststore.item(row_index).and_downcast::<VideoSegment>().unwrap();
        let row_count = split_table_liststore.n_items();
        for i in 0..video_player_count {
            let video_player = video_player_container.child_at_index(i)
                .and_then(|child| child.child())
                .and_downcast::<VideoPlayer>()
                .unwrap();
            let time = row.get_time_entry_copy(i as usize);
            let row_id = row_count - 1;
            video_player.connect_time_to_seekbar(format!("video-{i}, row-{row_id}"), time, "black");
        }
    }

    pub fn connect_column_to_seekbar(&self, video_player_container: &FlowBox, column_index: u32) {
        let imp = self.imp();
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore = match split_table_liststore_borrow.as_ref() {
            Some(ls) => ls,
            None => return,
        };

        let row_count = split_table_liststore.n_items();
        let video_player = video_player_container.child_at_index(column_index as i32)
            .and_then(|child| child.child())
            .and_downcast::<VideoPlayer>()
            .unwrap();
        for i in 0..row_count {
            let row = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let time = row.get_time_entry_copy(column_index as usize);
            video_player.connect_time_to_seekbar(format!("video-{column_index}, seg-{i}"), time, "black");
        }
    }

}
