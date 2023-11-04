use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;
use crate::jobs::{BuiltJob, JobEnvironment, JobError, JobResult, ResourceItem};
use crate::variables::context::read_context;
use crate::variables::evaluate::{evaluate, VariableEvalCounter};

// loads a resource from file to a string and throws an error if not found
pub fn resource_load(file: &Path, env: &JobEnvironment, built: &mut BuiltJob) -> JobResult<String> {
    let mut path = env.path.clone();
    path.push(file);

    let content = fs::read_to_string(&path).map_err(|e| JobError::Resources(file.to_owned(), e))?;
    // calculate checksum after reading, so the errors are more informative
    built.mark_resource(ResourceItem::create(file.to_owned(), &env.path)?);

    Ok(content)
}

// checks a resource whether it is a file or not
pub fn resource_dir(file: &Path, env: &JobEnvironment) -> JobResult<bool> {
    let mut path = env.path.clone();
    path.push(file);

    Ok(path.is_dir())
}

// checks that a resource exists and throws an error otherwise
pub fn resource_mark(file: &Path, env: &JobEnvironment, built: &mut BuiltJob) -> JobResult<()> {
    let mut path = env.path.clone();
    path.push(file);

    // check existence
    if !path.exists() {
        return Err(JobError::Resources(path.to_owned(), Error::new(ErrorKind::Other, "file does not exist")))
    }

    // TODO: Do this at the correct point in time, after the prompt
    built.mark_resource(ResourceItem::create(file.to_owned(), &env.path)?);

    Ok(())
}

// processes the variables inside a given string, and throws an error if it could not be resolved
pub fn process_variables(string: &str, path: &Path, env: &JobEnvironment, built: &mut BuiltJob) -> JobResult<String> {

    // parses the file and creates a context
    let context = read_context(string).map_err(|e| JobError::Variable(e, string.to_owned(), path.to_owned()))?;

    // evaluates the context
    let mut counter = VariableEvalCounter::default();
    let result = evaluate(string, &context, env.variables, &mut counter).map_err(|e| JobError::Variable(e, string.to_owned(), path.to_owned()))?;
    built.use_variables(counter);

    Ok(result)
}