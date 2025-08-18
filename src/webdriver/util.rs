use fancy_regex::Regex;
use chrono::NaiveTime;
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use thirtyfour::prelude::*;
use tokio::time::Duration;
use tokio::sync::Mutex;
use std::io;
use std::io::Write;
use super::process_manager;
use crate::course::course_manager::Course;
use super::scrape::get_term_table;
use rpassword::read_password;
use tokio::time;
use crate::Spinner;
use crate::VALID_SCL_DAYS;
use crate::CUSIS_COURSE_SEARCH_LINK;
use crate::CUSIS_LINK;
// a function to reformate scraped course data and store them as a Course struct 
pub fn data_formating(course_code: &str, data: &str, course_dict: &mut Arc<DashMap<String, Vec<Course>>>, course_vec: &mut Vec<Course>){
    let lines: Vec<&str> = data.lines().map(|l| l.trim()).collect(); // Collect lines into a Vec
    let mut class_code  = String::new();
    let mut tutorial_code = String::new();
    let mut lab_code = String::new();
    let mut datetime: HashMap<String, Vec<Vec<NaiveTime>>> = HashMap::new(); // Store day -> time slot
    let mut instructor = String::new();
    
    for (i, line) in lines.iter().enumerate() {
        if i == 1 && !line.starts_with("Open") {
            return; // Return if the course is not open
        }
        if line.contains("LEC") || line.contains("CLW") || line.contains("PRJ") {
            class_code = line.to_string();
        }
        else if line.contains("TUT"){
                tutorial_code = line.to_string();
        }
        else if line.contains("LAB"){
             lab_code = line.to_string();
        }
        
        if line.contains("Monday") || line.contains("Tuesday") || line.contains("Wednesday") ||
            line.contains("Thursday") || line.contains("Friday") || line.contains("Saturday") {
            // Check if there's a next line
            if i + 1 < lines.len() {
                let time_slot = lines[i + 1]; // Access the next line to get the time
                let parts: Vec<&str> = time_slot.split(" to ").collect(); // Split the time slot into start and end time
                let time_slot: Vec<NaiveTime> = match (
                NaiveTime::parse_from_str(parts[0], "%I:%M%P"),
                NaiveTime::parse_from_str(parts[1], "%I:%M%P"),
                ) {
                    (Ok(start_time), Ok(end_time)) => {
                        vec![start_time, end_time]
                    },
                    _ => {
                        eprintln!("Failed to parse times: {} - {}", parts[0], parts[1]);
                        vec![]
                    }
                };
                match *line {
                    l=>
                        for day in VALID_SCL_DAYS{
                            if l.contains(day){
                                match datetime.get_mut(day) {
                                    Some(data) => {
                                        if ! data.contains(&time_slot){
                                            data.push(time_slot.clone());
                                        }
                                    }
                                    None => {
                                        datetime.insert(day.to_string(), vec![time_slot.clone()]);
                                    }
                                }
                            }
                        }
                }
            }
        }
        else if line.contains("Mr.") || line.contains("Ms.") || line.contains("Dr.") || line.contains("Prof.") || line.contains("Professor"){
            instructor = line.to_string();
        }
        
        if !instructor.is_empty() && !datetime.is_empty() && !class_code.is_empty() {
            let course = Course::create_course_time(course_code.to_string(), datetime, instructor, class_code, tutorial_code, lab_code);
            course_vec.push(course);
            break;
        }
    }
    course_dict.insert(course_code.to_string(), course_vec.to_vec());
}

pub async fn wait_til_title(driver: &WebDriver, expected_title: &str) -> WebDriverResult<()>{
    tokio::time::timeout(Duration::from_secs(40), async{
      loop {
            let title = { driver.title().await? };
            if title == expected_title {
                break Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }) .await
    .map_err(|_| WebDriverError::Timeout(format!("Timed out waiting for title '{}'", expected_title)))?
}

pub async fn async_wait_til_title(driver: &Arc<Mutex<WebDriver>>, expected_title: &str) -> WebDriverResult<()> {
    println!("Waiting for {expected_title}");
    let driver_lock = driver.lock().await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let title = { driver_lock.title().await? };
            if title == expected_title {
                break Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .map_err(|_| WebDriverError::Timeout(format!("Timed out waiting for title '{}'", expected_title)))?
}

pub async fn search_and_click_element_with_retries(driver: &WebDriver, element: By, num_of_retries: i8, org_title: &Option<String>) -> WebDriverResult<()>{
    println!("{:?}", org_title);
    for i in 0..num_of_retries {
        match driver.find(element.clone()).await{
            Ok(found_element) => {
                if found_element.is_clickable().await?{
                    found_element.click().await?;

                    if let Some(title) = org_title {
                        let res = tokio::time::timeout(Duration::from_secs(3), async {
                            loop {
                                if let Ok(curr_driver_title) = driver.title().await{
                                    if *title != curr_driver_title {
                                        return Ok::<(), WebDriverError>(());
                                    }
                                    println!("{title}, {:?}", driver.title().await);
                                }
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }
                        }).await;
                        
                    }
                    else{
                       return Ok(());
                    }
                }
            },
            Err(e) => {
                println!("{e}");
                if let Some(title) = org_title {
                    if let Ok(curr_driver_title) = driver.title().await{
                        println!("{title}, {:?}", driver.title().await);
                        if *title != curr_driver_title {
                            println!("BREAK");
                            return Ok(());
                        }
                       
                    }
                }
                else{
                    if i < num_of_retries{
                        println!("retrying");
                        continue;
                    }
                }
                
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
    }
    Err(WebDriverError::HttpError("Failed to find and click the element after retries".to_string()))
}

pub async fn navigate_to_terms(driver: &WebDriver) -> WebDriverResult<(HashMap<String, (WebElement, String)>, WebElement)>{
    driver.goto(CUSIS_COURSE_SEARCH_LINK).await?;
    wait_til_title(driver, "Class Search and Enroll").await?;

    match driver.find(By::Id("DERIVED_SSR_FL_SSR_CSTRMPRV_GRP")).await{
        Ok(element)=> element.click().await?,
        Err(err)=> {println!("{}", err)}
    };

    let mut terms: HashMap<String, (WebElement, String)> = HashMap::new();
    get_term_table("prev", &driver, &mut terms, &None, false).await?;

    let curr_term =  driver.find(By::Id("DERIVED_SSR_FL_SSR_CSTRMCUR_GRP")).await?;
    curr_term.click().await?;

    get_term_table("curr", &driver, &mut terms, &None, false).await?;
    Ok((terms, curr_term))
}

pub async fn init_driver() ->  WebDriverResult<WebDriver> {
    let mut caps = DesiredCapabilities::firefox();
    //let _ = caps.set_headless();
    if cfg!(windows){
        let _ = match caps.set_firefox_binary(r"win64\130.0.1\firefox.exe"){
        Ok(caps) => caps,
        Err(e) => return Err(e)
        };
    }
    else{
        todo!()
        // linux and mac 
    }
   
    let driver = WebDriver::new("http://127.0.0.1:4444", caps).await?;
    Ok(driver)
}

// pub async fn select_school_terms(terms: HashMap <String, (WebElement, String)>, curr_term: WebElement)-> WebDriverResult<(WebElement, String)>{
//     let term_names: Vec<String> = terms.keys().cloned().collect();
//     loop {
//         println!("---------------------------------------------------------------------------------------------------------------------------");
//         println!("Index: Term");
//         for (index, term ) in term_names.iter().enumerate() {
//             println!("{index}: {term}");
//         }
//         println!("---------------------------------------------------------------------------------------------------------------------------");
//         println!("Please select the term you are scheduling your courses for (by the number): ");
//         let mut input = String::new();
//         io::stdin()
//         .read_line(&mut input)
//         .expect("Failed");

//         let index: usize = input.trim().parse().expect("Please enter a valid number.");
//         if let Some(term) = term_names.get(index){
//             if let Some((term_element, element_id)) = terms.get(term){
//                 if term_element.is_clickable().await?{
//                     term_element.click().await?;
//                     break Ok((term_element.clone(), element_id.clone()))
//                 }
//                 else {
//                     curr_term.click().await?;
//                     term_element.click().await?;
//                     break Ok((term_element.clone(), element_id.clone()))
//                 }
//             }
//         }
//         else{
//             println!("Please enter a valid number.");
//         }
//     }
// }

pub async fn select_school_terms(mut terms: HashMap <String, (WebElement, String)>, selected_term: String, driver: &WebDriver)-> WebDriverResult<()>{
    
    if let Some(term_data) = terms.get(&selected_term){
        let term_button = &term_data.0;
        match term_button.is_clickable().await{ 
            Ok(_) => {
                println!("clicked");
                term_button.click().await?;
                return Ok(());
            },
            Err(_) => {
                match driver.find(By::XPath("//*[text()='Change']")).await{
                    Ok(change_button)=> {
                        driver.enter_default_frame().await;
                        change_button.click().await?;
                        println!("pressed");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        if let Ok(iframe) = driver.find(By::Css("[title=\"Class Search and Enroll Popup window\"]")).await{
                            iframe.enter_frame().await?;
                            match search_and_click_element_with_retries(driver, By::Id("DERIVED_SSR_FL_SSR_CSTRMPRV_GRP"), 5, &Option::None).await{
                            Ok(()) => {
                                if let Ok(()) = get_term_table("prev", driver, &mut terms, &Option::Some(selected_term.clone()), true).await{
                                    driver.enter_default_frame().await;
                                    return Ok(());
                                }
                            },
                            Err(e)=> {
                                println!("{e}");
                            }
                        }
                        match search_and_click_element_with_retries(driver, By::Id("DERIVED_SSR_FL_SSR_CSTRMCUR_GRP"), 5,  &Option::None).await{
                        Ok(())=> {
                            if let Ok(()) = get_term_table("curr", driver, &mut terms, &Option::Some(selected_term.clone()), true).await{
                                driver.enter_default_frame().await;
                                return Ok(());
                            } 
                        },
                        Err(e) => {println!("{e}")}
                        }
                    }
                        
                    },
                    Err(e) => {

                        if let Ok(iframe) = driver.find(By::Css("[title=\"Class Search and Enroll Popup window\"]")).await{
                            iframe.enter_frame().await?;
                            match search_and_click_element_with_retries(driver, By::Id("DERIVED_SSR_FL_SSR_CSTRMPRV_GRP"), 5, &Option::None).await{
                            Ok(()) => {
                                if let Ok(()) = get_term_table("prev", driver, &mut terms, &Option::Some(selected_term.clone()), true).await{
                                    driver.enter_default_frame().await;
                                    return Ok(());
                                }
                            },
                            Err(e)=> {
                                println!("{e}");
                            }
                        }
                        match search_and_click_element_with_retries(driver, By::Id("DERIVED_SSR_FL_SSR_CSTRMCUR_GRP"), 5, &Option::None).await{
                        Ok(())=> {
                            if let Ok(()) = get_term_table("curr", driver, &mut terms, &Option::Some(selected_term.clone()), true).await{
                                driver.enter_default_frame().await;
                                return Ok(());
                            } 
                        },
                        Err(e) => {println!("{e}")}
                        }
                    }
                }
                
            }
           
        }

    }
    
    
    }
    driver.enter_default_frame().await;
    Err(WebDriverError::HttpError("Failed to select school term".to_string()))
}

pub async fn save_course_data(rows: Result<WebElement, WebDriverError>, course: String, course_collection: &mut Arc<DashMap<String, Vec<Course>>>) -> WebDriverResult<()> {
    let mut course_time = String::new();
    if let Ok(data) =  rows{
        course_time = data.text().await?.trim().to_string();
    }
    if course_time.len() == 0{
        println!("\n{} currently does not have any open class.", course);
        return Ok(())
    }
    if let Ok(re) = Regex::new(r"(?m)^\s*(\d+)\s*$"){
        let splits: Vec<Result<&str, fancy_regex::Error>> = re.split(&course_time).collect();
        let mut course_vec : Vec<Course> = Vec::new();

        for (_, section) in splits.iter().enumerate(){
            if let Ok(section) = section {
                data_formating(&course, *section,  course_collection, &mut course_vec);
            }
        }
        println!("\n{} has been looked up successfully", course);
    }
    
    Ok(())
}

pub async fn prompt_for_courses() -> String{
    println!("You can now enter the list of courses you are planning to take in the selected semester (seperated by space): " );
    let mut courses = String::new();
    io::stdin()
        .read_line(&mut courses)
        .expect("Failed");
    courses
}

pub async fn cleanup_driver(driver: Arc<WebDriver>) -> WebDriverResult<()>{
    match Arc::try_unwrap(driver) {
        Ok(driver) => {
            if let Err(e) = driver.quit().await {
                eprintln!("Failed to quit driver: {}", e);
            }
            if let Err(e) = process_manager::terminate_process("geckodriver.exe") {
                eprintln!("Failed to terminate geckodriver: {}", e);
            }
        }
        Err(_) => {
            eprintln!("Failed to unwrap Arc<WebDriver>: Multiple references exist, cannot quit driver cleanly");
        }
    }
    Ok(())
}

pub async fn goto_cusis(driver: &WebDriver)-> WebDriverResult<()>{
    // Timeout for page navigation
    time::timeout(Duration::from_secs(10), driver.goto(CUSIS_LINK)).await
        .map_err(|_| WebDriverError::Timeout("Failed to load CUSIS login page".to_string()))??;

    // Wait for the Sign In page
    time::timeout(Duration::from_secs(5), wait_til_title(driver, "Sign In")).await
        .map_err(|_| WebDriverError::Timeout("Timed out waiting for Sign In page".to_string()))??;
    Ok(())
}

pub async fn login(driver: &WebDriver, username: String, password: String) -> WebDriverResult<()> {
    let org_title = driver.title().await?;
    let user_name_field = time::timeout(Duration::from_secs(5), driver.find(By::Id("userNameInput"))).await
        .map_err(|_| WebDriverError::Timeout("Timed out finding username field".to_string()))??;
    let password_field = time::timeout(Duration::from_secs(5), driver.find(By::Id("passwordInput"))).await
        .map_err(|_| WebDriverError::Timeout("Timed out finding password field".to_string()))??;

    user_name_field.clear().await?;
    password_field.clear().await?;
    user_name_field.send_keys(&username).await?;
    password_field.send_keys(&password).await?;

    // Submit login
    let submit_button = time::timeout(Duration::from_secs(5), driver.find(By::Id("submitButton"))).await
        .map_err(|_| WebDriverError::Timeout("Timed out finding submit button".to_string()))??;
    submit_button.click().await?;
    time::sleep(Duration::from_secs(1)).await;
    if driver.title().await? == org_title{
        return Err(WebDriverError::HttpError("Incorrect user name or password".to_string()));
    }
    // Handle Duo Security 2FA
    
    if wait_til_title(driver, "Duo Security").await.is_ok() {
        let mut try_count = 0;
        loop{
            
            if let Ok(text_elem) = time::timeout(Duration::from_secs(5),driver.find(By::ClassName("align-text-horizontal-center"))).await{
                if let Ok(elem) = text_elem{
                    if let Ok(button) = elem.find(By::Css("button")).await {
                        button.click().await?;
                        break;
                    };
                }
            }
            
            try_count += 1;
            if try_count > 5 {
               break; 
            }
            time::sleep(Duration::from_secs(2)).await;
        }
    }
    else{
        return Err(WebDriverError::HttpError("Login failed".to_string()));
    } 

    Ok(())

}

// handles_auth(driver).await
    //     .map_err(|e| WebDriverError::Timeout("Timed out waiting for DUO".to_string()))?;
    // // Verify successful login
    // time::timeout(Duration::from_secs(10), wait_til_title(driver, "Homepage")).await
    //     .map_err(|_| WebDriverError::Timeout("Timed out waiting for Homepage after login".to_string()))??;
pub async fn handles_auth(driver: &WebDriver)-> WebDriverResult<()>{
    
    let spinner = Spinner::new(
        "Please complete Two-Factor Authtication on your device ".to_string(),
        vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'] ,
        100
    );
    spinner.start_spin();
    tokio::time::timeout(Duration::from_secs(120), async{
        loop {
            match driver.find(By::ClassName("try-again-button")).await {
                Ok(element) => {
                    element.click().await?;
                    print!("Please retry!");
                },
                Err(_) => {
                    if let Ok(button) = driver.find(By::Id("dont-trust-browser-button")).await{
                        button.click().await?;
                        spinner.stop();
                        break Ok(())
                    }
                    time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }).await.map_err(|_| WebDriverError::Timeout("Timed out waiting for title Duo Two-Factor Authtication".to_string()))?
}

pub fn get_user_data() -> (String, String){
    println!("Please enter your Student ID: ");
    let mut username = String::new();
   
    io::stdin()
        .read_line(&mut username)
        .expect("Failed");

    println!("Password: (Inputs are hidden)");
    if let Err(e) = std::io::stdout().flush(){
        eprintln!("Failed to flush output, {}", e);
    }
    let password = read_password()
        .expect("Failed");
    (username + "@link.cuhk.edu.hk", password)
}