use std::sync::Arc;
use super::util::*;
use tokio::sync::Mutex;
use thirtyfour::prelude::*;
use std::collections::HashMap;
use crate::CUSIS_COURSE_SEARCH_LINK;
use dashmap::DashMap;
use crate::course::course_manager::Course;
use crate::cli::animation::Spinner;

#[derive(Clone)]
pub struct CourseSearchTask{
    pub course: String,
    pub term: String,
    pub code: Option<Vec<String>>
}

// a function to scrape the term table for both previous and current terms
pub async fn get_term_table(
    cond: &str,
    driver: &WebDriver,
    terms: &mut HashMap <String, (WebElement, String)>,
    selected_term: &Option<String>,
    error_flag: bool
) -> Result<(), WebDriverError>{

    let title = match cond {
        "prev" => "[title=\"Previous Terms\"]",
        _ => "[title=\"Current Terms\"]"
    };
    match driver.find(By::Css(title)).await{
        Ok(element) => {
            let rows = element.find_all(By::Tag("tr")).await?;
            for term in rows {
                match term.attr("onclick").await?{
                    Some(id) => {
                        let term_text = term.text().await?;
                        
                        if term_text.contains("-"){
                            match selected_term{
                                Some(selected_term_name) => {
                                    if *selected_term_name == term_text{
                                        term.click().await?;
                                        return Ok(());
                                    }
                                    },
                                None => {
                                    terms.insert(term_text, (term, id));
                                }
                            }
                        }
                    },
                    None =>  {
                        if error_flag{
                            return Err(WebDriverError::HttpError("Id cannot be found".to_string()));
                        }
                    }
                };
            }
        },
        Err(err)=> {return  Err(err);}
    };
    Ok(())
}

// async fn open_new_tab(driver: Arc<Mutex<WebDriver>>) -> Option<WindowHandle>{
//     let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
//     if let Err(e) = driver_lock.new_tab().await{
//         return None
//     }
//     if let Ok(new_wins) = driver_lock.windows().await{
//         new_wins.into_iter().last().clone();
//     }
//     None
// }

pub async fn process_search_tasks(
    mut rx: tokio::sync::mpsc::Receiver<CourseSearchTask>,
    driver: Arc<Mutex<WebDriver>>,
    mut course_collection: Arc<DashMap<String, Vec<Course>>>,
    enroll: bool
)-> WebDriverResult<()>{
    // Consumer will process each search task one by one 
    while let Some(task) = rx.recv().await{
        let course = task.course;
        let term_id = task.term;
        
        //Open a new tab for each search task 
        let new_tab_handle = {
            let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
            driver_lock.new_tab().await?;
            driver_lock.windows().await?.into_iter().last().clone().unwrap()
        };
        // Navigate to Class Search Tab
        {
            let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
            driver_lock.switch_to_window(new_tab_handle.clone()).await?;
            if driver_lock.title().await? != "Class Search and Enroll" {
                driver_lock
                    .goto(CUSIS_COURSE_SEARCH_LINK)
                    .await?;
            }
        }
        async_wait_til_title(&driver, "Class Search and Enroll").await?;
        // Select the correct term for search 
        {
            let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
            if let Err(_) = driver_lock.find(By::Id("DERIVED_SSR_FL_SSR_CHANGE_BTN")).await{
                let term_dropdown_button = driver_lock.find(By::Id("DERIVED_SSR_FL_SSR_CSTRMPRV_GRP")).await?;
                println!("{:?}", term_dropdown_button);
                term_dropdown_button.click().await?;

                term_dropdown_button.click().await?;
                match driver_lock.find(By::XPath(format!("//a[text()='{}']", term_id))).await{
                    Ok(element) => {
                        element.click().await?
                    },
                    Err(e) => {
                    term_dropdown_button.click().await?;
                        if let Ok(element) = driver_lock.find(By::XPath(format!("//a[text()='{}']", term_id))).await{
                            element.click().await?;
                        }
                        else{
                            println!("Failed to click term button");
                        }
                    }
                };
            }
            
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        // Conduct search by filling in the search bar with the course code
        {
            let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
            let fill_search_bar_with_course_js = format!("document.getElementById('PTS_KEYWORDS3').value = '{}';", course.clone());
            driver_lock
            .execute(&fill_search_bar_with_course_js, vec![])
            .await?;
            let title = driver_lock.title().await?;
            search_and_click_element_with_retries(&driver_lock, By::Id("PTS_SRCH_BTN"), 5, &Option::Some(title)).await?;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        async_wait_til_title(&driver, "Class Search Results").await?;
        {
            let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
            let _ = match driver_lock.find(By::Css("#PTS_LIST_TITLE\\$0")).await{
                Ok(element)=> {
                    println!("{element}");
                    match element.attr("href").await {
                        Ok(Some(link)) => {
                            let owned_link = link.to_owned();
                            driver_lock.goto(&owned_link).await?;
                            tokio::time::sleep(tokio::time::Duration::from_micros(500)).await;
                            while driver_lock.title().await? != "Course Information" {
                                driver_lock.goto(&owned_link).await?;
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                            }
                        },
                        Ok(None) => println!("{} cannot be found!", course),
                        Err(e) => return Err(e),
                    }
                },
                Err(e)=> {
                    println!("{course} cannot be found in this semester, please make sure the course you are searching are open in this semester! ");
                    return Err(e);
                }
            };
        }
        async_wait_til_title(&driver, "Course Information").await?;
        if !enroll{
            let rows = {
                let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
                driver_lock.find(By::ClassName("ps_grid-body")).await
            };
            save_course_data(rows,  course, &mut course_collection).await?;
        }
        else{
            let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
            if let Some(codes_to_be_searched) = task.code{
                let text;
                if codes_to_be_searched.len() > 1 {
                    text = format!("//*[text()='{}']", codes_to_be_searched[1]);
                }
                else{
                    text = format!("//*[text()='{}']", codes_to_be_searched[0]);
                }
                match driver_lock.find(By::XPath(text)).await{
                    Ok(mut element) => {
                        loop {
                            let parent = element.parent()
                            .await?;
                            if parent.tag_name().await? == "tr"{
                                break
                            }
                            element = parent;
                        }
                        if element.is_clickable().await?{
                            element.click().await?;
                            wait_til_title(&driver_lock, "Review Class Selection").await?;
                            search_and_click_element_with_retries(&driver_lock, By::XPath("//*[text()='Next']"), 10, &Option::None).await?;
                            search_and_click_element_with_retries(&driver_lock, By::XPath("//*[text()='Accept']"), 10, &Option::None).await?;
                            search_and_click_element_with_retries(&driver_lock, By::XPath("//*[text()='Next']"), 10, &Option::None).await?;
                            search_and_click_element_with_retries(&driver_lock, By::XPath("//*[text()='Submit']"), 10, &Option::None).await?;
                            search_and_click_element_with_retries(&driver_lock, By::XPath("//*[text()='Yes']"), 10, &Option::None).await?;
                            // tokio::time::sleep(tokio::time::Duration::from_micros(500)).await;
                            // driver_lock.find(By::XPath("//*[text()='Submit']")).await?.click().await?;
                            // tokio::time::sleep(tokio::time::Duration::from_micros(500)).await;
                            // driver_lock.find(By::XPath("//*[text()='Yes']")).await?.click().await?;
                            let title = driver_lock.title().await?;
                            search_and_click_element_with_retries(&driver_lock, By::XPath("//*[text()='Return to Keyword Search Page']"), 10, &Option::Some(title)).await?;
                            println!("{} has been added to shopping cart!", course);
                        }
                    },
                    Err(e) => return Err(e)
                }
            }
        }
        {
            let driver_lock: tokio::sync::MutexGuard<'_, WebDriver> = driver.lock().await;
            driver_lock.switch_to_window(new_tab_handle.clone()).await?;
            driver_lock.close_window().await?;
            let handles = driver_lock.windows().await?;
            if let Some(first_handle) = handles.first() {
                driver_lock.switch_to_window(first_handle.clone()).await?;
            }
        }
        // if let Some(spinner) = spinner_option{
        //     spinner.stop();
        // }
        
    }
    Ok(())
}