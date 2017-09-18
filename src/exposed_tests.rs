// Determine which values of type Test are exposed from a given module.

use std::fs::File;
use std::io::{BufReader, BufRead};
use io;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

#[derive(Debug)]
pub enum Problem {
    UnexposedTests(String, HashSet<String>),
    MissingModuleDeclaration(PathBuf),
    OpenFileToReadExports(PathBuf, io::Error),
    ReadingFileForExports(PathBuf, io::Error),
    ParseError(PathBuf),
}

pub fn filter_exposing(
    path: &Path,
    tests: &HashSet<String>,
    module_name: &str,
) -> Result<(String, HashSet<String>), Problem> {
    let new_tests: HashSet<String> = match read_exposing(path)? {
        // None for exposed_values means "the module was exposing (..), so keep everything"
        None => tests.clone(),
        // Only keep the tests that were exposed.
        Some(exposed_values) => {
            exposed_values
                .intersection(&tests)
                .cloned()
                .collect::<HashSet<String>>()
        }
    };

    if new_tests.len() < tests.len() {
        Err(Problem::UnexposedTests(
            module_name.to_owned(),
            tests
                .difference(&new_tests)
                .cloned()
                .collect::<HashSet<String>>(),
        ))
    } else {
        Ok((module_name.to_owned(), new_tests))
    }
}

enum ParsedLineResult {
    AllExposed,
    Exposing(HashSet<String>, bool),
}

fn read_exposing(path: &Path) -> Result<Option<HashSet<String>>, Problem> {
    let file = File::open(path).map_err(|err| {
        Problem::OpenFileToReadExports(path.to_path_buf(), err)
    })?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut exposing: HashSet<String> = HashSet::new();

    loop {
        reader.read_line(&mut line).map_err(|err| {
            Problem::OpenFileToReadExports(path.to_path_buf(), err)
        })?;

        match parse_line(&line) {
            Ok(ParsedLineResult::AllExposed) => {
                return Ok(None);
            }
            Ok(ParsedLineResult::Exposing(new_exposing, is_done)) => {
                for val in new_exposing {
                    exposing.insert(val);
                }

                if is_done {
                    return Ok(Some(exposing));
                }
            }
            Err(_) => {
                return Err(Problem::ParseError(path.to_path_buf()));
            }
        }
    }
}

fn parse_line(line: &str) -> Result<ParsedLineResult, ()> {
    return Err(());
}

/* Remove all the comments from the line,
   and return whether we are still in a multiline comment or not
*/
fn strip_comments(line: &mut str, is_in_comment: bool) -> bool {
    loop {
        // when we have a single line comment
        if let Some(single_line_comment_index) = line.find("--") {
            if !is_in_comment {
                unsafe {
                    line.slice_mut_unchecked(0, single_line_comment_index);
                }
                continue;
            }
        }

        let block_comment_start = line.find("{-");
        let block_comment_end = line.find("-}");

        match (block_comment_start, block_comment_end) {
            // when there's a start and end
            (Some(start_index), Some(end_index)) => {
                // We know these indices will be okay because we got them from find()
                unsafe {
                    line.slice_mut_unchecked(0, start_index);
                }

                // Subtract start_index because the line just got shorter by that much.
                let dest_index = (end_index + 2) - start_index;
                let line_length = line.len();

                // We know these indices will be okay because we got them from find()
                unsafe {
                    line.slice_mut_unchecked(dest_index, line_length - dest_index);
                }
            }

            // when there's a start, but no end
            (Some(start_index), None) => {
                // We know these indices will be okay because we got them from find()
                unsafe {
                    line.slice_mut_unchecked(0, start_index);
                }

                return true;
            }

            // when there's an end, but no start
            (None, Some(end_index)) => {
                if is_in_comment {
                    let dest_index = end_index + 2;
                    let line_length = line.len();

                    // We know these indices will be okay because we got them from find()
                    unsafe {
                        line.slice_mut_unchecked(dest_index, line_length - dest_index);
                    }
                }

                return false;
            }

            // when there are no block comment chars
            (None, None) => {
                if is_in_comment {
                    // We know these indices will be okay because they're both 0.
                    unsafe {
                        line.slice_mut_unchecked(0, 0);
                    }
                }

                return is_in_comment;
            }
        }
    }
}

// Returns whether it found and removed a module declaration
fn remove_module_declaration(line: &str) -> &str {
    if line.starts_with("module") {
        let start_index = 6;
        let end_index = line.len();
        unsafe { line.slice_unchecked(start_index, end_index) }
    } else if line.starts_with("port module") {
        let start_index = 11;
        let end_index = line.len();
        unsafe { line.slice_unchecked(start_index, end_index) }
    } else if line.starts_with("effect module") {
        let start_index = 13;
        let end_index = line.len();
        unsafe { line.slice_unchecked(start_index, end_index) }
    } else {
        line
    }
}

#[cfg(test)]
mod test_remove_module_declaration {
    use super::*;

    #[test]
    fn removes_module() {
        let line = "module Foo exposing (blah)".to_owned();

        assert_eq!(" Foo exposing (blah)", remove_module_declaration(&line));
    }

    #[test]
    fn removes_port_module() {
        let line = "port module Bar exposing (blah)".to_owned();

        assert_eq!(" Bar exposing (blah)", remove_module_declaration(&line));
    }

    #[test]
    fn removes_effect_module() {
        let line = "effect module Baz exposing (blah)".to_owned();

        assert_eq!(" Baz exposing (blah)", remove_module_declaration(&line));
    }

    #[test]
    fn does_nothing_if_no_module() {
        let line = "blah blah whatever".to_owned();

        assert_eq!("blah blah whatever", remove_module_declaration(&line));
    }
}

// fn split_exposing(line: &str) -> HashSet<String> {
//     line.substr(0, exposingLine.lastIndexOf(")"))
//         .split(",")
//         .map(str::trim)
//         .collect<HashSet<String>>()
// }


// fn parse(reader: &mut BufReader) {
//   // if we're currently in a comment
//   let mut is_in_comment = false;
//
//   // if the file does not have a module line
//   let mut is_missing_module_name = false;
//
//   // if we're done parsing
//   let mut parsing_done = false;
//
//   // if the module line has been read
//   let mut has_module_line_been_read = false;
//
//   let mut is_reading_module_name = false;
//   let mut is_reading_exports = false;
//   let mut is_between_parens = false;
//
//   // values that have been exposed
//   let mut exposed_values = vec![];
//
//   // number of open/closed parens seen so far
//   let mut open_parens_seen = 0;
//   let mut closed_parens_seen = 0;
//
//   // data between exposing brackets
//   let mut data = "";
//
//   fn parse_line(line: &mut str) {
//     if parsing_done { return; }
//
//     is_in_comment = stripComments(line, isInComment);
//     line = line.trim();
//
//     if line.is_empty() { return; }
//
//     // if we haven't started reading the first line
//     if !has_module_line_been_read {
//         let new_line = remove_module_declaration(line);
//         if new_line == line {
//         // We did not find a module to remove, meaning we found content before the module
//         // declaration. Error!
//           is_missing_module_name = true;
//           parsing_done = true;
//
//           return;
//         }
//       } else {
//             // We found and successfully removed the module declaration.
//           has_module_line_been_read = true;
//           is_reading_module_name = true;
//
//           if line.is_empty() {return;}
//     }
//
//     // if we are still reading the module line
//     if is_reading_module_name {
//       match line.find("exposing") {
//           Some(exposing_index) => {
//             let line_length = line.len();
//               let dest_index = exposing_index + 8;
//                 unsafe {
//                     line.slice_mut_unchecked(dest_index, line_length - dest_index);
//                 }
//               is_reading_module_name = false;
//               is_reading_exports = true;
//
//               if line.is_empty() {return;}
//           },
//           None => { return; }
//       }
//     }
//
//     // if we are actually reading the exports
//     if is_reading_exports {
//         match line.find("(") {
//             Some(first_paren_index) => {
//               open_parens_seen += 1;
//               is_reading_exports = false;
//               is_between_parens = true;
//
//                 let line_length = line.len();
//                   let dest_index = first_paren_index + 1;
//
//
//                 unsafe {
//                     line.slice_mut_unchecked(dest_index, line_length - dest_index);
//                 }
//             }
//             None => { return; }
//         }
//     }
//
//     // if we're before the final bracket
//     if is_between_parens {
//       let new_open_parens_seen = line.split("(").len();
//       let new_closed_parens_seen = line.split(")").len();
//
//       closed_parens_seen += new_closed_parens_seen;
//       open_parens_seen += new_open_parens_seen;
//
//       data += line;
//
//       if closedBracketsSeen == openBracketsSeen {
//         exposed_values = split_exposing(data);
//         parsing_done = true;
//       }
//     }
//   }
// }
