use crate::items::{InputItem, OutputItem};

pub(crate) fn get_new_response(output: &[OutputItem]) -> Vec<OutputItem> {
    output.to_vec()
}

pub(crate) fn run_single_turn(prepared_input: &[InputItem]) -> &[InputItem] {
    prepared_input
}

pub(crate) fn run_single_turn_streamed(prepared_input: &[InputItem]) -> &[InputItem] {
    prepared_input
}
