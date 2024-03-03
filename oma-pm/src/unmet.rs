use oma_apt::raw::cache::raw::Version;
use resolvo_deb::resolvo::DefaultSolvableDisplay;
use resolvo_deb::DebSolver;

pub fn find_unmet_deps(pkgs: Vec<Version>) -> String {
    let mut debs = DebSolver::new_local().unwrap();
    let requirment = debs.get_requirement(pkgs).unwrap();
    
    match debs.solve(requirment) {
        Ok(_solvables) => "No Problem".to_string(),
        Err(problem) => {
            problem
                .display_user_friendly(&debs.0, &DefaultSolvableDisplay)
                .to_string()
        }
    }
}
