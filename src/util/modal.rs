use anyhow::anyhow;
use itertools::Itertools as _;
use serenity::all::{InputText, ModalInteraction};

pub(crate) struct ModalInputTexts {
    inputs: Vec<InputText>,
}

impl ModalInputTexts {
    pub(crate) fn new(ixn: &ModalInteraction) -> Self {
        let inputs = ixn
            .data
            .components
            .iter()
            .filter_map(|row| {
                let item = row.components[0].clone();
                match item {
                    serenity::all::ActionRowComponent::InputText(item) => Some(item),
                    _ => None,
                }
            })
            .collect_vec();
        Self { inputs }
    }

    pub(crate) fn get_value(&self, input_id: &str) -> anyhow::Result<Option<String>> {
        self.inputs
            .iter()
            .find(|input| input.custom_id == input_id)
            .ok_or(anyhow!("unexpected missing input {input_id}"))
            .map(|input| input.value.clone())
    }

    pub(crate) fn get_required_value(&self, input_id: &str) -> anyhow::Result<String> {
        let value = self.get_value(input_id)?;
        value.ok_or(anyhow!("Expected value for {input_id} was missing"))
    }
}
