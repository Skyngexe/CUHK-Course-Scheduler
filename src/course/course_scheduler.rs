use crate::VALID_SCL_DAYS;

use super::course_manager::Course;
use dashmap::DashMap;
use core::time;
//use tokio::time;
use std::{collections::{HashMap, HashSet}};
use chrono::NaiveTime;
#[derive(Debug)]
#[derive(Clone)]
pub struct Scheduler {
    scheduled_courses_name: Vec<String>,
    time_slot: HashMap<String,  Vec<Vec<NaiveTime>>>,
    scheduled_course_details: HashSet<Course>,
    candidate_solutions: Vec<(i64, HashSet<Course>)>,
    index: i64
}
impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            scheduled_courses_name: vec![],
            time_slot: 
                crate::VALID_SCL_DAYS.iter()
                .map(|&day| (day.to_string(), Vec::new()))
                .collect(),
            scheduled_course_details: HashSet::new(),
            candidate_solutions: vec![],
            index: 0
        }
    }

    fn find_timetable_index(time_str: String, day: &str) -> i32{
        let time_parts = time_str.split(":").collect::<Vec<&str>>();
        let index = time_parts[0].parse::<i32>().unwrap() - 9;
        match day{
            "Monday" => {
                return index * 6 + 1;
            },
            "Tuesday" => {
                return index * 6 + 2;
            },
            "Wednesday" => {
                return index * 6 + 3;
            },
            "Thursday" => {
                return index * 6 + 4;
            },
            "Friday" => {
                return index * 6 + 5;
            },
            "Saturday" => {
                return index * 6 + 6;
            },
            _ =>{
                return -1;
            }
        }
        
    } 
    fn reduce_course_set_to_timetable_string(course_set: &HashSet<Course>) -> Vec<String>{
        let mut res = vec![String::from(""); 84];
        // use the res to store the class data in the format "Course code\nTime\nInstructor"
        for course in course_set{
            let timeslot = &course.datetime;
            let instructor = &course.instructor;
            let course_name= &course.course_name;
            for day in VALID_SCL_DAYS{
                if let Some(class_time_vec) = timeslot.get(day){
                    for class_time in class_time_vec{
                        let mut course_data_str = String::from("");
                        let start_time = class_time[0].to_string();
                        course_data_str.push_str(course_name);
                        course_data_str.push_str("\n");
                        course_data_str.push_str(&class_time[0].format("%H:%M").to_string());
                        course_data_str.push_str(&" - ");
                        course_data_str.push_str(&class_time[1].format("%H:%M").to_string());
                        course_data_str.push_str("\n");
                        course_data_str.push_str(instructor);
                        let index = Scheduler::find_timetable_index(start_time, day);
                        res[index as usize] = course_data_str;
                    }
                }
            }
        }
        res
    }
    pub fn get_next_schedule(&mut self, direction: i8) -> Option<(Vec<String>, Vec<(String, Vec<String>)>)>{
        if self.index >= 0 && self.index < self.candidate_solutions.len() as i64 && self.candidate_solutions.len() > 0 {
            let schedule = &self.candidate_solutions[self.index as usize].1;
            let result = Some((Scheduler::reduce_course_set_to_timetable_string(schedule), self.reduce_course_set_to_course_and_choice_vec(&schedule)));
            match direction{
                1 => {
                    self.index += 1
                },
                0 => {
                    self.index -= 1
                },
                _ => {}
            }
           result
        }
        else{
            None
        }
    }
    fn transform_course_set(&self, solution: &HashSet<Course>) -> HashMap<String,  Vec<Vec<NaiveTime>>>{
        let mut temp_time_slot: HashMap<String,  Vec<Vec<NaiveTime>>> =  
            crate::VALID_SCL_DAYS.iter()
            .map(|&day| (day.to_string(), Vec::new()))
            .collect();
        for course in solution{
            let date_time: &HashMap<String, Vec<Vec<NaiveTime>>> = &course.datetime;
            for (day, class_period) in date_time{
                for class_time in class_period{
                    let start_time = &class_time[0];
                    temp_time_slot
                    .entry(day.to_string())
                    .and_modify(|slots| {
                        let pos = slots
                            .binary_search_by_key(&start_time, |slot| &slot[0])
                            .unwrap_or_else(|i| i);
                        slots.insert(pos, class_time.clone());
                    })
                    .or_insert_with(|| class_period.clone());
                }
            }
        }
        temp_time_slot
    }

    fn cal_fitness_score(&self, solution: &HashSet<Course>, day_off_preference: String) -> i64{
        let mut score = 0;
        let time_slot = self.transform_course_set(solution);
        match time_slot.get(&day_off_preference){
            Some(courses) => {
                score +=  (courses.len())as i64 * 100;
            },
            None => {
                score -= 200;
            }
        }
        let vec: Vec<Vec<Vec<NaiveTime>>> = time_slot.into_values().into_iter().filter(|x|x.len() > 0).collect();
        
        for day in vec{
            if day.len() > 1{
                for i in 1..day.len(){
                    let last_period = &day[i-1];
                    let current_period =   &day[i];
                    let time_gap_between_course = current_period[0] - last_period[1];
                    score += time_gap_between_course.num_minutes()
                }
            }
            else if day.len() == 1{
                score += 20;
            }
        }
        score
    }
   

    // a recursive backtracking algorithm 
    pub fn generate_schedule(&mut self, course_dict: &DashMap<String, Vec<Course>>, day_off_preference: String){

        if self.scheduled_courses_name.len() == course_dict.len(){
            if !self.candidate_solutions.iter().any(|&(_, ref set)| set == &self.scheduled_course_details){
                let solution_score = self.cal_fitness_score(&self.scheduled_course_details.clone(), day_off_preference.clone());
                let index = self.candidate_solutions.binary_search_by_key(&solution_score, |score|score.0)
                .unwrap_or_else(|i|i);
                self.candidate_solutions.insert(index, (solution_score, self.scheduled_course_details.clone()));
                return;
            }
        }
        
        // Loop throught the course_name-course key_pair 
        for entry in course_dict.iter(){
            let (course_name, course) = entry.pair();
            // For each course, loop through the possible options
            if self.scheduled_courses_name.contains(course_name){
                continue; // Skip already scheduled courses
            }
            // i tells us which course option we have gone through 
            for i in 0..course.len(){
                // check if the course is already in schedule
                let option =  &course[i];
                let timeslot_dictionary = option.datetime.clone();
                // For each couse option, there are several daytime combination which the program has to make sure all of them can fit into the schedule conflict-free
                let mut can_schedule = true;
                for (date, class_time_period)in &timeslot_dictionary{
                        match self.time_slot.get(date){
                        Some(occupied_timeslots) => {
                            // if there is no conflicting class, add the class to the time_slot
                            if !self.check_availability(&occupied_timeslots, &class_time_period){
                                can_schedule = false;
                            }
                        },
                        None => {}
                        };
                }

                if can_schedule{
                    for (day, class_time_period) in &timeslot_dictionary {
                        if let Some(schedule) = self.time_slot.get_mut(day){
                            for class in class_time_period{
                                schedule.push(class.clone());
                            }
                        }
                        self.scheduled_courses_name.push(course_name.clone());
                        self.scheduled_course_details.insert(option.clone());
                        self.generate_schedule(course_dict, day_off_preference.clone()); 
                        self.scheduled_courses_name.pop();
                        self.scheduled_course_details.remove(option);
                        for (day, class_time_period) in &timeslot_dictionary {
                            for scheduled_timeslot in class_time_period {
                                if let Some(org_timeslot) = self.time_slot.get_mut(day){
                                    if org_timeslot.iter().any(|time| time == scheduled_timeslot){
                                        org_timeslot.retain(|time| time != scheduled_timeslot);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
   
    fn check_availability(
        &self,
        occupied_timeslots: &Vec<Vec<NaiveTime>>,
        new_timeslots: &Vec<Vec<NaiveTime>>,
    ) -> bool {
        for new_slot in new_timeslots {
            let new_start = new_slot[0];
            let new_end = new_slot[1];
            for occupied_slot in occupied_timeslots {
                let occupied_start = occupied_slot[0];
                let occupied_end = occupied_slot[1];
                if new_start < occupied_end && new_end > occupied_start {
                    return false;
                }
            }
        }
        true
    }

    fn reduce_course_set_to_course_and_choice_vec(&self, course_set: &HashSet<Course>) -> Vec<(String, Vec<String>)>{
        // A function that takes a HashSet of Course and reduce it into pairs of (course_name, code)    
        // the return value will then be used for enrolling courses
        course_set.iter()
        .map(
            |course|
            {
                let mut codes = vec![course.class_code.clone()];
                if !course.lab_code.is_empty() {
                    codes.push(course.lab_code.clone());
                }
                if !course.tutorial_code.is_empty() {
                    codes.push(course.tutorial_code.clone());
                }
                (course.course_name.clone(), codes)
            }
        ).collect()
    }

    pub fn get_schedule_with_best_fitness_score(&self) -> Option<Vec<(String, Vec<String>)>>{
        if self.candidate_solutions.len() > 0 {
            println!("\nBest Generated Schedule: {:?}", &self.candidate_solutions[0].1);
            //println!("{:?}", self.transform_course_set(&self.candidate_solutions[0].1));
            Some(self.reduce_course_set_to_course_and_choice_vec(&self.candidate_solutions[0].1))
        }
        else{
            None
        }
    }
    // pub fn get_next_schedule(&self) -> Option<Vec<(String, Vec<String>)>>{

    // }
}


#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    #[test]
    fn test_backtracking_schedulling_algorithm(){
        let course_collection_hashmap = HashMap::from(
            [
                (
                "CSCI3180", 
                vec![
                    Course {   
                        course_name: "CSCI3180".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Monday".to_string(), vec![vec![NaiveTime::from_hms_opt(14,30,00).unwrap(), NaiveTime::from_hms_opt(16,15,00).unwrap()]]),
                                ("Tuesday".to_string(), vec![
                                    vec![NaiveTime::from_hms_opt(15,30,00).unwrap(), NaiveTime::from_hms_opt(16,15,00).unwrap()], 
                                    vec![NaiveTime::from_hms_opt(17,30,00).unwrap(), NaiveTime::from_hms_opt(18,15,00).unwrap()]
                                    ]
                                )
                            ]   
                        ), 
                        instructor: "Professor Lauren Marcelyn PICK".to_string(), 
                        class_code: "( 8232 ) - - LEC".to_string(), tutorial_code: "( 8810 ) -T01 - TUT".to_string(), lab_code: "".to_string() 
                    }, 
                    Course { 
                        course_name: "CSCI3180".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Tuesday".to_string(), vec![vec![NaiveTime::from_hms_opt(15,30,00).unwrap(), NaiveTime::from_hms_opt(16,15,00).unwrap()]]), 
                                ("Thursday".to_string(), vec![vec![NaiveTime::from_hms_opt(12,30,00).unwrap(), NaiveTime::from_hms_opt(13,15,00).unwrap()]]), 
                                ("Monday".to_string(), vec![vec![NaiveTime::from_hms_opt(14,30,00).unwrap(), NaiveTime::from_hms_opt(16,15,00).unwrap()]])
                            ]
                        ), 
                        instructor: "Professor Lauren Marcelyn PICK".to_string(), 
                        class_code: "( 8232 ) - - LEC".to_string(), tutorial_code: "( 8188 ) -T03 - TUT".to_string(), lab_code: "".to_string()
                    }, 
                    Course { 
                        course_name: "CSCI3180".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Wednesday".to_string(), vec![vec![NaiveTime::from_hms_opt(16,30,00).unwrap(), NaiveTime::from_hms_opt(17,15,00).unwrap()]]), 
                                ("Monday".to_string(), vec![vec![NaiveTime::from_hms_opt(14,30,00).unwrap(), NaiveTime::from_hms_opt(16,15,00).unwrap()]]), 
                                ("Tuesday".to_string(), vec![vec![NaiveTime::from_hms_opt(15,30,00).unwrap(), NaiveTime::from_hms_opt(16,15,00).unwrap()]])
                            ]
                        ), 
                        instructor: "Professor Lauren Marcelyn PICK".to_string(), 
                        class_code: "( 8232 ) - - LEC".to_string(), tutorial_code: "( 8885 ) -T02 - TUT".to_string(), lab_code: "".to_string() 
                    }
                ]
                ),
                ( 
                "CSCI3100", 
                vec![
                    Course { 
                        course_name: "CSCI3100".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Tuesday".to_string(), vec![vec![NaiveTime::from_hms_opt(12,30,00).unwrap(), NaiveTime::from_hms_opt(14,15,00).unwrap()]]), 
                                ("Monday".to_string(), vec![vec![NaiveTime::from_hms_opt(11,30,00).unwrap(), NaiveTime::from_hms_opt(12,15,00).unwrap()], vec![NaiveTime::from_hms_opt(16,30,00).unwrap(), NaiveTime::from_hms_opt(17,15,00).unwrap()]])
                            ]
                        ), 
                        instructor: "Dr. LAM Tak Kei".to_string(), 
                        class_code: "( 8249 ) - - LEC".to_string(), tutorial_code: "( 8853 ) -T01 - TUT".to_string(), lab_code: "".to_string()
                    }, 
                    Course { 
                        course_name: "CSCI3100".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Tuesday".to_string(), vec![vec![NaiveTime::from_hms_opt(12,30,00).unwrap(), NaiveTime::from_hms_opt(14,15,00).unwrap()]]), 
                                ("Monday".to_string(), vec![vec![NaiveTime::from_hms_opt(11,30,00).unwrap(), NaiveTime::from_hms_opt(12,15,00).unwrap()]]), 
                                ("Wednesday".to_string(), vec![vec![NaiveTime::from_hms_opt(17,30,00).unwrap(), NaiveTime::from_hms_opt(18,15,00).unwrap()]])
                            ]
                        ), 
                        instructor: "Dr. LAM Tak Kei".to_string(), 
                        class_code: "( 8249 ) - - LEC".to_string(), tutorial_code: "( 8208 ) -T03 - TUT".to_string(), lab_code: "".to_string() 
                    }, 
                    Course { 
                        course_name: "CSCI3100".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Tuesday".to_string(), vec![vec![NaiveTime::from_hms_opt(12,30,00).unwrap(), NaiveTime::from_hms_opt(14,15,00).unwrap()]]), 
                                ("Monday".to_string(), vec![vec![NaiveTime::from_hms_opt(11,30,00).unwrap(), NaiveTime::from_hms_opt(12,15,00).unwrap()], vec![NaiveTime::from_hms_opt(17,30,00).unwrap(), NaiveTime::from_hms_opt(18,15,00).unwrap()]])
                            ]
                        ), 
                        instructor: "Dr. LAM Tak Kei".to_string(), 
                        class_code: "( 8249 ) - - LEC".to_string(), tutorial_code: "( 8034 ) -T02 - TUT".to_string(), lab_code: "".to_string() 
                    }
                ]
                ), 
                (
                "UGEA2163", 
                vec![
                    Course { 
                        course_name: "UGEA2163".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Friday".to_string(), vec![vec![NaiveTime::from_hms_opt(09,30,00).unwrap(), NaiveTime::from_hms_opt(11,15,00).unwrap()]])
                            ]
                        ),
                        instructor: "Dr. LAU Po Hei".to_string(), 
                        class_code: "( 8255 ) - - LEC".to_string(), tutorial_code: "".to_string(), lab_code: "".to_string() 
                    }
                ]
                ), 
                ( 
                "ELTU3502", 
                vec![
                    Course { 
                        course_name: "ELTU3502".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Monday".to_string(), vec![vec![NaiveTime::from_hms_opt(12,30,00).unwrap(), NaiveTime::from_hms_opt(14,15,00).unwrap()]])
                            ]
                        ),
                        instructor: "Ms. LEUNG Kit Chi Ella".to_string(), 
                        class_code: "( 4980 ) BC01 - CLW".to_string(), tutorial_code: "".to_string(), lab_code: "".to_string() 
                    }, 
                    Course { 
                        course_name: "ELTU3502".to_string(),
                        datetime: HashMap::from(
                            [
                                ("Thursday".to_string(), vec![vec![NaiveTime::from_hms_opt(10,30,00).unwrap(), NaiveTime::from_hms_opt(12,15,00).unwrap()]])
                            ]
                        ), 
                        instructor: "Ms. LEUNG Kit Chi Ella".to_string(), 
                        class_code: "( 9663 ) CC01 - CLW".to_string(), tutorial_code: "".to_string(), lab_code: "".to_string() 
                    }
                ]
                ),
                (
                    "CSCI3250", 
                    vec![
                        Course { 
                            course_name: "CSCI3250".to_string(),
                            datetime: HashMap::from(
                                [
                                    ("Thursday".to_string(), vec![vec![NaiveTime::from_hms_opt(13,30,00).unwrap(), NaiveTime::from_hms_opt(15,15,00).unwrap()]])
                                ]
                            ), 
                            instructor: "Dr. Umair Mujtaba QURESHI".to_string(), 
                            class_code: "( 9085 ) - - LEC".to_string(), tutorial_code: "".to_string(), lab_code: "".to_string() 
                        }
                    ]
                )
        ]
        );
    let course_collection: &DashMap<String, Vec<Course>> = &DashMap::new();
    for (key, value) in course_collection_hashmap {
        course_collection.insert(key.to_string(), value);
    }    

    let mut scheduler = Scheduler::new();
    scheduler.generate_schedule(&(*course_collection).clone(), "Thursday".to_string());
    scheduler.get_schedule_with_best_fitness_score();
    assert!(scheduler.candidate_solutions.len() > 0);
    }
}
