use std::collections::HashMap;
use chrono::NaiveTime;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Eq, PartialEq)]  
pub struct Course{
    //the struct owns the values instead of a reference 
    pub course_name: String,
    pub datetime: HashMap<String, Vec<Vec<NaiveTime>>>,
    pub instructor: String,
    pub class_code: String,
    pub tutorial_code: String,
    pub lab_code: String
}

impl Hash for Course {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.course_name.hash(state);
        self.instructor.hash(state);
        self.class_code.hash(state);
        self.tutorial_code.hash(state);
        self.lab_code.hash(state);
    }
}
 
impl Course{
    pub fn create_course_time(
        course_name: String,
        datetime: HashMap<String, Vec<Vec<NaiveTime>>>, 
        instructor: String, 
        class_code: String, 
        tutorial_code: String,
        lab_code: String
    ) -> Course {
        Course {
            course_name,
            datetime,
            instructor,
            class_code,
            tutorial_code,
            lab_code
        }
    }
}
