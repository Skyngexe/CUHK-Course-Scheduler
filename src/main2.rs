use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc::channel;
use thirtyfour::{prelude::*};
use dashmap::DashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use webdriver::{process_manager, scrape, util};
mod course;
mod webdriver;
mod cli;
use course::course_scheduler::Scheduler;
use course::course_manager::Course;
use scrape::{*, CourseSearchTask};
use util::*;
use cli::animation::Spinner;

const CUSIS_LINK: &str = "https://cusis.cuhk.edu.hk/psp/CSPRD/?cmd=login&languageCd=ENG&";
const CUSIS_COURSE_SEARCH_LINK: &str = "https://cusis.cuhk.edu.hk/psc/CSPRD_4/EMPLOYEE/SA/c/SSR_STUDENT_FL.SSR_CLSRCH_MAIN_FL.GBL?Page=SSR_CLSRCH_MAIN_FL";
const GECKODRIVER_PORT: &str = "4444";
const VALID_SCL_DAYS: [&str; 6] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

//#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn init() -> WebDriverResult<()> {
    let _child = process_manager::GeckodriverGuard(process_manager::spawn_process(cfg!(windows), &["./geckodriver.exe", "--port", GECKODRIVER_PORT]).unwrap());
    let driver = util::init_driver().await?;
    
    let result = async {
        login(&driver).await?;
        util::wait_til_title(&driver, "Homepage").await?;
        
        //After loggining in successfully, we can navigate to the "Manage Class" page
        println!("You have successfully logged into CUSIS!");
        let term_id: String;
        match navigate_to_terms(&driver).await{
            Ok((terms, curr_term))=> {
            (_, term_id) = select_school_terms(terms, curr_term).await?;
                let courses = prompt_for_courses().await;
                // should probably add constrain on the num of courses...
                // Arc is used for reference counting to allow multiple ownership, ensuring the underlying data is not dropped until all references are gone
                let course_arc: Arc<Vec<String>> = Arc::new(courses.split_whitespace().map(String::from).collect()); // for concurrent read of course data
                let driver_arc: Arc<Mutex<WebDriver>> = Arc::new(Mutex::new(driver.clone()));
                let course_collection: Arc<DashMap<String, Vec<Course>>> = Arc::new(DashMap::new());
                let (tx, rx) = channel::<CourseSearchTask>(10);
                //Consumer thread: Process each search task one by one
                let consumer_handle = tokio::spawn(
                    process_search_tasks(
                        rx,
                        driver_arc, 
                        course_collection.clone(),
                        false
                    )
                );
                // Producer threads 
                let ops: Arc<AtomicU64> = Arc::new(AtomicU64::new(0)); // Atomic Counter for fetching course 
                let mut handles = vec![];
                for _ in 0..course_arc.len() {
                    let tx_clone = tx.clone();
                    let ops_clone = ops.clone();
                    let course_clone = course_arc.clone();
                    let term_id = term_id.clone();
                    let code = Option::None;
                    handles.push(tokio::spawn(async move {
                        let index = ops_clone.fetch_add(1, Ordering::SeqCst);
                        let course = course_clone[index as usize].clone(); 
                        if let Err(e) = tx_clone.send(CourseSearchTask{course, term_id, code}).await{
                            eprintln!("Failed to send Course Search Task {}", e);
                        }
                    }));
                }
                
                for handle in handles {
                    let _ = handle.await;
                };

                drop(tx);
                if let Err(e) = consumer_handle.await {
                    eprintln!("Consumer task join failed: {:?}", e);
                }
                let mut scheduler = Scheduler::new();
            
                scheduler.generate_schedule(&(*course_collection).clone());
                if let Some(courses_to_be_enrolled) = scheduler.get_schedule_with_best_fitness_score(){
                    let course_code_to_be_enrolled: Vec<String> = courses_to_be_enrolled.iter().map(|data|data.0.clone()).collect();
                    let codes: Vec<Vec<String>> = courses_to_be_enrolled.iter().map(|data|data.1.clone()).collect();
                    let driver_arc: Arc<Mutex<WebDriver>> = Arc::new(Mutex::new(driver.clone()));
                    let (tx, rx) = channel::<CourseSearchTask>(10);
                    let consumer_handle = tokio::spawn(
                        process_search_tasks(
                            rx,
                            driver_arc, 
                            course_collection.clone(),
                            true
                        )
                    );
                    // Producer threads 
                    let ops: Arc<AtomicU64> = Arc::new(AtomicU64::new(0)); // Atomic Counter for fetching course 
                    let mut handles = vec![];
                    for _ in 0..course_code_to_be_enrolled.len() {
                        let tx_clone = tx.clone();
                        let ops_clone = ops.clone();
                        let course_clone = course_code_to_be_enrolled.clone();
                        let term_id = term_id.clone();
                        let codes = codes.clone();
                        handles.push(tokio::spawn(async move {
                            let index = ops_clone.fetch_add(1, Ordering::SeqCst);
                            let course = course_clone[index as usize].clone(); 
                            let code = codes[index as usize]. clone();
                            if let Err(e) = tx_clone.send(CourseSearchTask{course, term_id, code: Some(code)}).await{
                                eprintln!("Failed to send Course Search Task {}", e);
                            }
                        }));
                    }
                    
                    for handle in handles {
                        let _ = handle.await;
                    };

                    drop(tx);
                    if let Err(e) = consumer_handle.await {
                        eprintln!("Consumer task join failed: {:?}", e);
                    }
                }
                else{
                    println!("Sorry no possible schedule can be generated!");
                }
            },

            Err(e)=> {
                println!("Error: {}", e)
            }
        };
        Ok::<(), Box<dyn std::error::Error>>(())
    }.await;
    if let Err(e) = result {
        eprintln!("Error encountered: {:?}", e);
    }
    cleanup_driver(driver).await;
    Ok(())
}















