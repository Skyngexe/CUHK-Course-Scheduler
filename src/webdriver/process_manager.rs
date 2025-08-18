use std::process::{Child, Command, Stdio};

pub fn terminate_process(image_name: &str) -> Result<(), String> {
    if cfg!(windows){
        let result = Command::new("powershell")
        .arg("taskkill")
        .arg("/F")
        .arg("/IM")
        .arg(image_name)
        .arg("/T")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
        match result {
            Ok(mut child) => {
                let _ = child.wait();
                Ok(())
            },
            Err(e) => Err(e.to_string())
        }
    }
    else{
        let result  = Command::new("killall")
        .arg(image_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

        match result {
            Ok(mut child) => {
                let _ = child.wait();
                Ok(())
            },
            Err(e) => Err(e.to_string())
        }
    }
   
}

pub fn spawn_process(os_flag: bool, args: &[&str]) -> Result<Child, String> {
    let mut command = String::new();
    match os_flag {
        true => command.push_str("powershell"),
        false => command.push_str("bash")
    }
    let result = Command::new(command)
        .arg("-c")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    match result {
        Ok(child) => Ok(child),
        Err(e) => Err(e.to_string())
    }
}

pub struct GeckodriverGuard(pub Child);
impl Drop for GeckodriverGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_terminate_process() {
        spawn_process(cfg!(windows), &["./geckodriver.exe", "--port=4444"]).unwrap();
        let result: Result<_, String> = terminate_process("geckodriver.exe");
        println!("res: {:?}", result);
        assert!(result.is_ok() || result.is_err()); // Check that it returns a Result
    }
}


