use crate::error::UserFacingError;
use crate::schema;

#[derive(cynic::QueryVariables, Debug)]
pub struct ListAiTerminalDevImagesVariables {}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "RootQuery",
    variables = "ListAiTerminalDevImagesVariables"
)]
pub struct ListAiTerminalDevImages {
    #[cynic(rename = "listAiTerminalDevImages")]
    pub list_ai_terminal_dev_images: ListAiTerminalDevImagesResult,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ListAiTerminalDevImagesOutput {
    pub images: Vec<ImageTag>,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ImageTag {
    pub image: String,
    pub repository: String,
    pub tag: String,
}

#[derive(cynic::InlineFragments, Debug)]
pub enum ListAiTerminalDevImagesResult {
    ListAiTerminalDevImagesOutput(ListAiTerminalDevImagesOutput),
    UserFacingError(UserFacingError),
    #[cynic(fallback)]
    Unknown,
}

crate::client::define_operation! {
    ListAiTerminalDevImages(ListAiTerminalDevImagesVariables) -> ListAiTerminalDevImages;
}
