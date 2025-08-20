<div align="center">

# CUHK Course Scheduler

<div align="center">
<img src="https://raw.githubusercontent.com/marwin1991/profile-technology-icons/main/icons/rust.png" alt="Rust Logo" width="70"/>
<img src="https://raw.githubusercontent.com/slint-ui/slint/master/logo/slint-logo-full-dark.svg" alt="Slint Logo" width="100"/>
</div>

</div>


CUHK Course Scheduler is a ***cross-platform desktop application*** designed to simplify course scheduling for CUHK students. This lightweight desktop app automates course data retrieval from the CUSIS portal, generates a range of optimized, conflict-free schedules, and adds courses directly to your shopping cart. 

## 🌟 Why I Built CUHKScheduler
Manually scheduling courses on the CUSIS portal was a hassle that ate up hours each semester. 
I had to search for course codes one by one, check time compatibility, and ensure the schedule fit my day-off preferences, like avoiding early classes or securing free days. This tedious process inspired me to build CUHK Course Scheduler, an app that eliminates the frustration of course planning by automating  the manual task of searching, planning and adding, saving time and effort.
## 🚀 Features
- **Automated Course Data Scraping**: Fetches course details from the CUSIS portal using concurrent, asynchronous processing for fast and reliable data collection with a single driver to maximize throughput and minimize system resource usage 
- **Smart Schedule Generation**: Uses recursive backtracking with pruning and greedy local search to produce multiple conflict-free schedules tailored to preferences like avoiding early classes or maximizing free days.
- **CUSIS Shopping Cart Integration**: Automatically adds selected courses to the CUSIS shopping cart, streamlining enrollment.
- **Intuitive Slint UI**: Features a sleek, cross-platform interface with seamless login, term selection, course input, and schedule visualization.
- **Secure Single-User Design**: Each app instance runs locally, feeding login credentials directly to the CUSIS portal’s login page without a traditional backend, enhancing security by avoiding credential storage or server-side processing.
- **Customizable Preferences**: Supports manual course entry, term selection from backend data, and preferences like preferred times or days off.
- **Cross-Platform Support**: Runs on Windows, macOS, and Linux, with managed dependencies like geckodriver and Firefox for easy setup.

## 🛠️ Technology Stack
- Rust: Powers the business logic with high performance, memory safety, and concurrency

- Slint: Drives a responsive, cross-platform UI

- Consumer-Producer Threading and Channels: Implements a consumer-producer model with Rust’s channels for full-stack integration, enabling efficient, thread-safe communication between the Slint frontend and Rust backend for seamless data flow and UI updates.

- Tokio: Enables asynchronous scraping and task management

- Thirtyfour: Facilitates reliable WebDriver automation for CUSIS interaction

- Geckodriver & Firefox: Ensures robust web scraping with automated setup.

## 🧠 Design Decision
- **Single vs. Multiple WebDrivers for Scraping**: A key architectural decision was whether to use a single WebDriver or multiple WebDrivers for scraping course data from the CUSIS portal. Using multiple WebDrivers could parallelize scraping tasks but would increase resource usage (CPU, memory, and browser instances), potentially impacting performance on user devices. It is also more difficult to manage different login sessions at the same time. 
I chose a single WebDriver to minimize resource consumption, ensuring efficiency and reliability for the single-user desktop app. 

- **Transitioning from CLI to User-Friendly GUI**: Initially, this project was designed as a command-line interface (CLI) tool, which was efficient but potentially intimidating for non-technical CUHK students. Recognizing the need to resonate with a broader user base, I pivoted to building a Slint-based graphical user interface (GUI) to make the app more accessible and intuitive. That's why there's a cli folder within src, just for record keeping.


## 🌟 Get Involved
I am preparing for the **full release** and invite CUHK students to try CUHK Course Scheduler in **October**! 
If you are interested in participating in closed beta testing,please let me know!


## 📅 Roadmap
- **Closed Beta** (Late August 2025): Limited release to selected CUHK students for testing core features and gathering feedback. Set up Launch Page.

- **Feedback Integration** (September 2025): Incorporate user feedback to fix bugs, refine the Slint UI, and enhance features like preference customization.

- **Open Beta** (Early October 2025): Expand access to a broader group for final testing and polishing. Create a launch video to showcase CUHKScheduler’s capabilities and plan for future enhancements like schedule previews, export options, and additional preferences (e.g., professor or location constraints).

- **General Release** (Late October 2025): Launch the fully stable version of CUHK Course Scheduler for all CUHK students.


