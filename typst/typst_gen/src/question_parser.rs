use std::fmt::Write;

use color_eyre::eyre::eyre;
use verifier::{
    api::Question,
    markdown::{Answers, parse},
};

pub struct TypstWriter;

impl TypstWriter {
    pub fn typst_from_markdown(markdown: &str) -> color_eyre::Result<String> {
        let questions = parse(markdown).map_err(|e| eyre!("Error parsing markdown: {e}"))?;
        let mut buf = String::new();
        buf.write_str(
            r#"
#set text(
  font: "Palatino",
)
"#,
        )?;
        Self::write_md(&questions, &mut buf)?;
        Ok(buf)
    }
    #[allow(dead_code)]
    pub fn write(questions: &[Question], buf: &mut impl Write) -> color_eyre::Result<()> {
        writeln!(buf, "= Survey Questions and Answers\n")?;
        for question in questions {
            Self::write_question(question, buf)?;
        }
        Ok(())
    }
    pub fn write_md(
        questions: &[verifier::markdown::Question],
        buf: &mut impl Write,
    ) -> color_eyre::Result<()> {
        writeln!(buf, "= Survey Questions and Answers\n")?;
        for question in questions {
            Self::write_md_question(question, buf)?;
        }
        Ok(())
    }
    fn write_md_question(
        question: &verifier::markdown::Question,
        buf: &mut impl Write,
    ) -> color_eyre::Result<()> {
        writeln!(buf, "== {}\n", question.text)?;
        match &question.answers {
            Answers::Matrix {
                label1,
                answers1,
                answers2,
            } => {
                writeln!(buf, "Type: matrix\n")?;
                writeln!(buf, "Tag: {label1}")?;
                writeln!(buf, "Rows:\n")?;
                for row in answers1 {
                    writeln!(buf, "- {row}")?;
                }
                writeln!(buf, "\nColumns:\n")?;
                for col in answers2 {
                    writeln!(buf, "- {col}")?;
                }
            }
            Answers::FreeForm => {
                writeln!(buf, "Type: free form")?;
            }
            Answers::SelectOne(variants) => {
                writeln!(buf, "Type: select one")?;
                for variant in variants {
                    writeln!(buf, "- {variant}")?;
                }
            }
            Answers::SelectMany(variants) => {
                writeln!(buf, "Type: select all that apply")?;
                for variant in variants {
                    writeln!(buf, "- {variant}")?;
                }
            }
            Answers::RatingScale { .. } => {
                writeln!(buf, "Type: rating scale\n")?;
            }
            Answers::Ranking(rankings) => {
                writeln!(buf, "Type: ranking\n")?;
                for variant in rankings {
                    writeln!(buf, "- {variant}")?;
                }
            }
            Answers::InputList(inputs) => {
                writeln!(buf, "Type: input list\n")?;
                for input in inputs {
                    writeln!(buf, "- {input}")?;
                }
            }
        }
        writeln!(buf)?;
        Ok(())
    }

    fn write_question(
        question: &verifier::api::Question,
        buf: &mut impl Write,
    ) -> color_eyre::Result<()> {
        writeln!(buf, "== {}\n", question.text())?;
        if !question.description_text().is_empty() {
            writeln!(buf, "{}\n", question.description_text())?;
        }
        match question {
            Question::Input { .. } => {
                writeln!(buf, "Type: free form")?;
            }
            Question::ChoiceList { choice_list, .. } => {
                if question.is_select_one() {
                    writeln!(buf, "Type: select one")?;
                } else {
                    writeln!(buf, "Type: select all that apply")?;
                }
                writeln!(buf)?;

                for variant in choice_list.as_strs() {
                    writeln!(buf, "- {variant}")?;
                }
            }
            Question::ChoiceTable { choice_table, .. } => {
                writeln!(buf, "Type: matrix\n")?;
                writeln!(buf, "Rows:\n")?;
                for row in choice_table.rows_strs() {
                    writeln!(buf, "- {row}")?;
                }
                writeln!(buf, "\nColumns:\n")?;
                for col in choice_table.column_strs() {
                    writeln!(buf, "- {col}")?;
                }
            }
            Question::RatingScale { .. } => {
                writeln!(buf, "Type: rating scale\n")?;
            }
            Question::Ranking { ranking, .. } => {
                writeln!(buf, "Type: ranking\n")?;
                for variant in ranking.as_strs() {
                    writeln!(buf, "- {variant}")?;
                }
            }
            Question::InputList { input_list, .. } => {
                writeln!(buf, "Type: input list\n")?;
                for input in input_list.as_strs() {
                    writeln!(buf, "- {input}")?;
                }
            }
        }
        writeln!(buf)?;
        Ok(())
    }
}
