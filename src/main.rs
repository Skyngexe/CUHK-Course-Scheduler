use fancy_regex::Regex;
use slint::spawn_local;
use slint::{PlatformError, SharedString, Weak, VecModel, ModelRc};
use thirtyfour::error::WebDriverErrorInner;
use std::f32::consts::LOG10_2;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::channel;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc::unbounded_channel;
use thirtyfour::{prelude::*};
use dashmap::DashMap;
use webdriver::{process_manager, scrape, util};
mod course;
mod webdriver;
mod cli;
use tokio::time::Duration;
use std::rc::Rc;
use tokio::time;
use course::course_scheduler::Scheduler;
use course::course_manager::Course;
use scrape::{*, CourseSearchTask};
use async_compat::{Compat, CompatExt};
use util::*;
use cli::animation::Spinner;

const CUSIS_LINK: &str = "https://cusis.cuhk.edu.hk/psp/CSPRD/?cmd=login&languageCd=ENG&";
const CUSIS_COURSE_SEARCH_LINK: &str = "https://cusis.cuhk.edu.hk/psc/CSPRD_4/EMPLOYEE/SA/c/SSR_STUDENT_FL.SSR_CLSRCH_MAIN_FL.GBL?Page=SSR_CLSRCH_MAIN_FL";
const GECKODRIVER_PORT: &str = "4444";
const VALID_SCL_DAYS: [&str; 6] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

slint::include_modules!();

fn main() -> Result<(), PlatformError> {
    // Create the Tokio runtime
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(5)
            .enable_all()
            .build()
            .map_err(|e| PlatformError::from(format!("Failed to create Tokio runtime: {}", e)))?
    );

    // Start Geckodriver
    let _child = process_manager::GeckodriverGuard(
        process_manager::spawn_process(cfg!(windows), &["./geckodriver.exe", "--port", GECKODRIVER_PORT])
            .map_err(|e| PlatformError::Other(format!("Failed to start Geckodriver: {}", e)))?,
    );

    // Initialize WebDriver
    let driver = rt.block_on(async {
        init_driver()
            .await
            .map_err(|e| PlatformError::Other(format!("WebDriverError: {}", e)))
    })?;
    let driver_reg = Arc::new(Mutex::new(driver.clone()));
    let driver_arc = Arc::new(driver);
    let terms_hashmap: Arc<std::sync::Mutex<Option<HashMap<String, (WebElement, String)>>>> = Arc::new(std::sync::Mutex::new(None));
    let terms_hashmap_clone = terms_hashmap.clone();
    let term_hashmap_use_clone = terms_hashmap.clone();
    // Navigate to CUSIS
    rt.block_on(async {
        goto_cusis(&driver_arc)
            .await
            .map_err(|e| PlatformError::Other(format!("Cannot connect to CUSIS: {}", e)))
    })?;

    // Create the UI instance
    let app = App::new()?;
    app.window().set_maximized(true);

    // Create a channel for login results
    let (tx, mut rx) = unbounded_channel::<String>();
    let driver_clone = driver_arc.clone();
    let login_ui_weak = app.as_weak();
    let driver_cleanup = driver_arc.clone();
    //let tx_reg_clone = tx.clone();
    let rt_clone = rt.clone();
    let rt_reg_clone = rt.clone();
    //let rt_sch_course_clone = rt.clone();
    let rt_course_clone = rt.clone();
    let reg_ui_weak = app.as_weak();
    let reg_ui_weak_clone = app.as_weak();
    // Handle login
    app.on_handle_login({
        let tx: tokio::sync::mpsc::UnboundedSender<String> = tx.clone();
        let driver = driver_arc.clone();
        let login_ui_weak = login_ui_weak.clone();

        move |student_id, password| {
            let tx = tx.clone();
            let driver = driver.clone();
            let login_ui_weak = login_ui_weak.clone();
            let student_id = format!("{}@link.cuhk.edu.hk", student_id);
            let password = password.to_string();

            // Update UI to show loading state
            slint::invoke_from_event_loop({
                let login_ui_weak = login_ui_weak.clone();
                move || {
                    if let Some(ui) = login_ui_weak.upgrade() {
                        ui.set_is_loading(true);
                        ui.set_message("Attempting login...".into());
                    }
                }
            }).unwrap();

            // Spawn login task in Tokio runtime to se
            rt_clone.spawn(async move {
                match login(&driver, student_id, password).await {
                    Ok(_) => {
                        tx.send("Login successfully".to_string()).unwrap();
                    }
                    Err(e) => {
                        tx.send(format!("Login failed: {}", e)).unwrap();
                    }
                }
            });
        }
    });

    let rt_clone = rt.clone();
    // Handle login result
    rt_clone.spawn(async move {
        while let Some(result) = rx.recv().await {
            let login_ui_weak = login_ui_weak.clone();
            let driver = driver_arc.clone();
            let tx = tx.clone();
            let rt_clone = rt.clone();
            let terms_hashmap_clone = terms_hashmap_clone.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(ui) = login_ui_weak.upgrade() {
                    ui.set_is_loading(false);
                    match result.as_str() {
                        "Login successfully" => {
                            let driver = driver.clone();
                            let tx = tx.clone();
                            let login_ui_weak = login_ui_weak.clone();
                            ui.set_message("Login successfully, please confirm your login session in DUO".into());
                            // Spawn DUO auth task
                            rt_clone.spawn(async move {
                                if let Err(e) = handles_auth(&driver).await {
                                    tx.send(format!("DUO auth failed: {}", e)).unwrap();
                                    return;
                                }
 
                                tokio::time::sleep(Duration::from_secs(5)).await;
                                // Navigate to terms
                                match navigate_to_terms(&driver).await {
                                    Ok((terms, _curr_term)) => {
                                        {
                                            let mut lock = terms_hashmap_clone.lock().unwrap();
                                            *lock = Some(terms.clone());
                                        }
                                        let terms_vec: Vec<SharedString> = terms.keys().map(|term| term.into()).collect();
                                        slint::invoke_from_event_loop(move || {
                                            if let Some(ui) = login_ui_weak.upgrade() {
                                                ui.set_current_page(Pages::Registration);
                                                ui.set_available_terms(ModelRc::new(VecModel::from(terms_vec)));
                                            }
                                        }).unwrap();
                                        

                                    }
                                    Err(e) => {
                                        println!("{e}");
                                        tx.send(format!("Error navigating to terms: {}", e)).unwrap();
                                    }
                                }
                            });
                        }
                        _ => {
                            ui.set_message(result.into());
                            ui.set_password("".into());
                        }
                    }
                }
            }).unwrap();
        }
    });
    let rt_reg_clone = rt_reg_clone.clone();
    let (scheduler_tx, mut scheduler_rx) = channel::<Scheduler>(10);
    let mut term_holder = Arc::new(Mutex::new(Option::None));
    let term_holder_clone = term_holder.clone();
    let term_holder_clone_use = term_holder.clone();
    app.on_term_selected({ 
        move |selected_value| {
            let term_hashmap_use_clone: Arc<std::sync::Mutex<Option<HashMap<String, (WebElement, String)>>>> = term_hashmap_use_clone.clone();
            let selected_value = selected_value.to_string();
            let driver_clone = driver_clone.clone();
            
            let mut term_holder_clone = term_holder_clone.clone();
            // Clone the terms hashmap out of the mutex before entering async context
            let terms_opt = {
                let lock = term_hashmap_use_clone.lock().unwrap();
                lock.as_ref().cloned()
            };
            rt_reg_clone.spawn(async move {
                let mut term_holder_clone = term_holder_clone.lock().await;
                *term_holder_clone = Some(selected_value.clone());              
                if let Some(terms) = terms_opt {
                    select_school_terms(terms, selected_value, &driver_clone).await;
                } else {
                    panic!()
                }
            });
        }
        
    });
    //let mut scheduler_holder: Arc<Mutex<Option<Scheduler>>> = Arc::new(Mutex::new(Option::None));
    let scheduler_holder_clone: Option<Scheduler> = Option::None;
    app.on_init_reg({
        // slint::invoke_from_event_loop(move || {
        // if let Some(ui) = reg_ui_weak_clone.upgrade() {
        //     ui.set_is_loading(true);}}).unwrap();
        let rt_course_clone = rt_course_clone.clone();
        let driver_reg_clone = driver_reg.clone();
        let term_holder_clone = term_holder_clone_use.clone();
        let scheduler_holder_clone = scheduler_holder_clone.clone();
        let scheduler_tx = scheduler_tx.clone();
        let reg_ui_weak = reg_ui_weak.clone();
        move |course: SharedString, day_off: SharedString| {
            let term_holder_clone = term_holder_clone.clone();
            let scheduler_holder_clone = scheduler_holder_clone.clone();
            let driver_reg_clone: Arc<Mutex<WebDriver>> = driver_reg_clone.clone();
            let scheduler_tx = scheduler_tx.clone();
            let reg_ui_weak = reg_ui_weak.clone();
            rt_course_clone.spawn(async move {
                let courses_to_search: Arc<Vec<String>> = Arc::new(course.split_whitespace().map(String::from).collect());
                let course_collection: Arc<DashMap<String, Vec<Course>>> = Arc::new(DashMap::new());
                let course_collection_clone = course_collection.clone();
                let (course_search_tx, course_search_rx) = channel::<CourseSearchTask>(10);
                // Consumer thread: Process each search task one by one
                let consumer_handle = tokio::spawn(
                    process_search_tasks(
                        course_search_rx,
                        driver_reg_clone.clone(),
                        course_collection_clone.clone(),
                        false
                    )
                );
                // Producer threads 
                let ops: Arc<AtomicU64> = Arc::new(AtomicU64::new(0)); // Atomic Counter for fetching course 

                let mut handles = vec![];
                for _ in 0..courses_to_search.len() {
                    let course_search_tx = course_search_tx.clone();
                    let ops_clone: Arc<AtomicU64> = ops.clone();
                    let course_clone: Arc<Vec<String>> = courses_to_search.clone();
                    let code = Option::None;
                    let term_holder_clone = term_holder_clone.clone();
                    handles.push(tokio::spawn(async move {
                        let term_holder_clone = term_holder_clone.lock().await;
                        if let Some(term) = term_holder_clone.as_ref(){
                            let index = ops_clone.fetch_add(1, Ordering::SeqCst);
                            let course = course_clone[index as usize].clone(); 
                            let term = term.to_string();

                            if let Err(e) = course_search_tx.send(CourseSearchTask{course, term, code}).await{
                                eprintln!("Failed to send Course Search Task {}", e);
                            }
                        }
                        
                    }));
                }
                for handle in handles {
                    let _ = handle.await;
                }
                drop(course_search_tx);
                if let Err(e) = consumer_handle.await {
                    eprintln!("Consumer task join failed: {:?}", e);
                }
                slint::invoke_from_event_loop(move || {
                if let Some(ui) = reg_ui_weak.upgrade() {
                ui.set_is_loading(false);}}).unwrap();
                let mut scheduler: Scheduler = Scheduler::new();
                scheduler.generate_schedule(&(*course_collection.clone()).clone(), day_off.to_string());
                scheduler_tx.send(scheduler).await;
              
            });
        }   
    }
);
    // if let Some(scheduler) = scheduler_holder_clone{
    //     scheduler.get_schedule_with_best_fitness_score();
    //         slint::invoke_from_event_loop({
    //             let timetable_ui_weak = timetable_ui_weak.clone();
    //             move || {
    //                 if let Some(ui) = timetable_ui_weak.upgrade() {
    //                     ui.set_current_page(Pages::TimeTable);
    //                 }
    //             }
    //         }).unwrap();
    //     }
    let timetable_ui_weak = app.as_weak();
    rt_clone.spawn(async move {
        while let Some(mut scheduler) = scheduler_rx.recv().await{
            let timetable_ui_weak = timetable_ui_weak.clone();
            
            slint::invoke_from_event_loop({
                let timetable_ui_weak = timetable_ui_weak.clone();
                move || {
                    if let Some(ui) = timetable_ui_weak.upgrade() {
                       ui.set_current_page(Pages::TimeTable);
                        if let Some(best_schedule) = scheduler.get_next_schedule(1){
                            println!("{:?}", best_schedule.0);
                            let best_schedule_vec: Vec<SharedString> = best_schedule.0.iter().map(|data| data.into()).collect();
                            let timetable_model = Rc::new(VecModel::from(best_schedule_vec));
                            ui.set_current_timetable(timetable_model.into());
                       }
                    }
                }
            }).unwrap();
            
    }});
   
//    app.on_get_next_schedule({
//     if let Some(best_schedule) = scheduler.get_next_schedule(1){
//                             println!("{:?}", best_schedule.0);
//                             let best_schedule_vec: Vec<SharedString> = best_schedule.0.iter().map(|data| data.into()).collect();
//                             let timetable_model = Rc::new(VecModel::from(best_schedule_vec));
//                             ui.set_current_timetable(timetable_model.into());
//                        }
//    });

//    app.on_get_prev_schedule({
    
//    });
    
    // Run the UI
    app.run()?;

    // Cleanup WebDriver
    rt_clone.block_on(async {
        cleanup_driver(driver_cleanup)
            .await
            .map_err(|e| PlatformError::Other(format!("WebDriverError: {}", e)))
    })?;

    Ok(())
}