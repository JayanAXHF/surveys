//! Converts SurveyHero markdown files to text files
//! with SurveyHero's text format
//! They can be then imported to SurveyHero directly, see https://help.surveyhero.com/manual/create-survey/how-to-copy-and-paste-questions-import-questions.
//!
//! NOTE: This is meant to be used to bootstrap
//! a survey; it is not meant to be a full solution
use crate::markdown::Question;
use anyhow::Result;
use std::io::Write;
use std::{io::Read, path::Path};

pub fn md_to_txt(source: &Path, dist: &Path) -> Result<()> {
    let mut file = std::fs::File::open(source)?;
    let mut contents = String::new();
    assert!(dist.is_dir());
    assert!(source.is_file());
    file.read_to_string(&mut contents)?;

    let questions = crate::markdown::parse(&contents)?;
    let dist_file = dist.join(source.with_extension(".txt").file_name().unwrap());
    let mut dist_file = std::fs::File::create(dist_file)?;
    write_questions(&questions, &mut dist_file)?;

    Ok(())
}

pub fn write_questions(questions: &[Question<'_>], out: &mut impl Write) -> Result<()> {
    for q in questions {
        write_question(q, out)?;
    }
    Ok(())
}

/// Writes a single question to the output
/// as per SurveyHero's text format defined
/// at https://help.surveyhero.com/manual/create-survey/how-to-copy-and-paste-questions-import-questions/
///
/// Limitations:
/// Can't parse InputList and Ranking
/// No control flow
/// Doesn't handle optionality (due to lack of support in the `surveyhero` crate)
fn write_question(q: &Question<'_>, out: &mut impl Write) -> Result<()> {
    use crate::markdown::Answers::*;
    write!(out, "{}", q.text)?;
    if !matches!(
        &q.answers,
        FreeForm | SelectMany(_) | InputList(_) | Matrix { .. }
    ) {
        writeln!(out)?;
    }
    match &q.answers {
        FreeForm => {
            write!(out, "+")?;
            writeln!(out, "\n")?;
        }
        RatingScale => {
            writeln!(out, "*****")?;
        }
        Ranking(a) => {
            write!(out, " [RANKING]")?;
            write_answers(a, out)?
        }
        SelectOne(a) => write_answers(a, out)?,
        SelectMany(a) => {
            write!(out, "+")?;
            writeln!(out, "\n")?;
            write_answers(a, out)?;
        }
        Matrix {
            label1: _,
            answers1,
            answers2,
        } => {
            writeln!(out, "+")?;
            writeln!(out, "{}", answers2.join("|"))?;
            write_answers(answers1, out)?;
        }
        InputList(a) => {
            write!(out, " [INPUT LIST] ")?;
            write!(out, "+")?;
            writeln!(out, "\n")?;
            write_answers(a, out)?;
        }
    }
    writeln!(out)?;
    out.flush()?;
    Ok(())
}

fn write_answers(answers: &[&str], out: &mut impl Write) -> Result<()> {
    for answer in answers.iter() {
        write!(out, "{}", answer)?;
        if answer.to_lowercase() == "other" {
            write!(out, "+")?;
        }
        writeln!(out)?;
    }
    Ok(())
}
