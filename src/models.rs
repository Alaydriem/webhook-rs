use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt::Display;
type Snowflake = String;

pub struct Interval<T> {
    pub max_allowed: T,
    pub min_allowed: T,
}

impl<T: Ord> Interval<T> {
    pub const fn from_min_max(min_allowed: T, max_allowed: T) -> Self {
        Interval {
            min_allowed,
            max_allowed,
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        self.min_allowed <= *value && self.max_allowed >= *value
    }
}

macro_rules! interval_member {
($name:ident, $option_inner_t:ty, $lower_bound:expr, $upper_bound:expr) => {
        pub(crate) const $name : Interval<$option_inner_t> = Interval::from_min_max($lower_bound, $upper_bound);
    };
}

#[derive(Deserialize, Debug)]
pub struct Webhook {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub webhook_type: i8,
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub token: String,
    pub application_id: Option<Snowflake>,
}

#[derive(Debug)]
pub(crate) struct MessageContext {
    custom_ids: HashSet<String>,
    embeds_character_counter: usize,
    button_count_in_action_row: usize,
}

fn interval_check<T: Ord + Display>(
    interval: &Interval<T>,
    value_to_test: &T,
    field_name: &str,
) -> Result<(), String> {
    if !interval.contains(value_to_test) {
        return Err(format!(
            "{} ({}) not in the [{}, {}] interval",
            field_name, value_to_test, interval.min_allowed, interval.max_allowed
        ));
    }
    Ok(())
}

impl MessageContext {
    /// Tries to register a custom id.
    ///
    /// # Watch out!
    ///
    /// Use only `register_button` for registering Buttons!
    ///
    /// # Arguments
    ///
    /// * `id`: the custom id to be registered
    ///
    ///
    /// # Return value
    /// Error variant contains an error message
    fn register_custom_id(&mut self, id: &str) -> Result<(), String> {
        interval_check(
            &Message::CUSTOM_ID_LEN_INTERVAL,
            &id.len(),
            "Custom ID length")?;

        if !self.custom_ids.insert(id.to_string()) {
            return Err(format!("Attempt to use the same custom ID ({}) twice!", id));
        }
        Ok(())
    }

    /// Tries to register an Embed
    ///
    /// # Return value
    ///
    /// None on no error. Some(String) containing the reason for failure.
    pub fn register_embed(&mut self, embed: &Embed) -> Result<(), String> {

        self.embeds_character_counter += embed.title.as_ref().map_or(0, |s| s.len());
        self.embeds_character_counter += embed.description.as_ref().map_or(0, |s| s.len());
        self.embeds_character_counter += embed.footer.as_ref().map_or(0, |f| f.text.len());
        self.embeds_character_counter += embed.author.as_ref().map_or(0, |a| a.name.len());

        embed.fields.iter().for_each(|f| {
            self.embeds_character_counter += f.name.len() + f.value.len();
        });

        interval_check(
            &Message::EMBED_TOTAL_TEXT_LEN_INTERVAL,
            &self.embeds_character_counter,
            "Character count across all embeds")?;
        Ok(())
    }

    pub(crate) fn new() -> MessageContext {
        MessageContext {
            custom_ids: HashSet::new(),
            button_count_in_action_row: 0,
            embeds_character_counter: 0
        }
    }

    /// Tries to register a button using the button's custom id.
    ///
    /// # Return value
    /// Error variant contains an error message
    ///
    /// # Note
    /// Subsequent calls register other components semantically in the same action row.
    /// To register components in a new action row, use the `register_action_row` function before
    /// calling this function
    fn register_button(&mut self, id: &str) -> Result<(), String> {
        self.register_custom_id(id)?;
        self.button_count_in_action_row += 1;

        interval_check(
            &ActionRow::BUTTON_COUNT_INTERVAL,
            &self.button_count_in_action_row,
            "Button count")?;
        Ok(())
    }
    /// Switches the context to register components logically in a "new" action row.
    ///
    /// # Watch out!
    /// This function shall be called only once per one action row. (due to the lack of action row
    /// identification)
    fn register_action_row(&mut self) {
        self.button_count_in_action_row = 0;
        self.button_count_in_action_row = 0;
    }
}

#[derive(Serialize, Debug)]
pub struct Message {
    pub content: Option<String>,
    pub thread_name: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub tts: bool,
    pub embeds: Vec<Embed>,
    pub allow_mentions: Option<AllowedMentions>,
    #[serde(rename = "components")]
    pub action_rows: Vec<ActionRow>,
}

impl Message {
    pub fn new() -> Self {
        Self {
            content: None,
            thread_name: None,
            username: None,
            avatar_url: None,
            tts: false,
            embeds: vec![],
            allow_mentions: None,
            action_rows: vec![],
        }
    }

    pub fn content(&mut self, content: &str) -> &mut Self {
        self.content = Some(content.to_owned());
        self
    }

    pub fn thread_name(&mut self, thread_name: &str) -> &mut Self {
        self.thread_name = Some(thread_name.to_owned());
        self
    }


    pub fn username(&mut self, username: &str) -> &mut Self {
        self.username = Some(username.to_owned());
        self
    }

    pub fn avatar_url(&mut self, avatar_url: &str) -> &mut Self {
        self.avatar_url = Some(avatar_url.to_owned());
        self
    }

    pub fn tts(&mut self, tts: bool) -> &mut Self {
        self.tts = tts;
        self
    }

    pub fn embed<Func>(&mut self, func: Func) -> &mut Self
    where
        Func: Fn(&mut Embed) -> &mut Embed,
    {
        let mut embed = Embed::new();
        func(&mut embed);
        self.embeds.push(embed);

        self
    }

    pub fn action_row<Func>(&mut self, func: Func) -> &mut Self
    where
        Func: Fn(&mut ActionRow) -> &mut ActionRow,
    {
        let mut row = ActionRow::new();
        func(&mut row);
        self.action_rows.push(row);

        self
    }

    interval_member!(ACTION_ROW_COUNT_INTERVAL, usize, 0, 5);
    interval_member!(LABEL_LEN_INTERVAL, usize, 0, 80);
    interval_member!(CUSTOM_ID_LEN_INTERVAL, usize, 1, 100);
    // Additionally, the combined sum of characters in all title, description, field.name,
    // field.value, footer.text, and author.name fields across all embeds attached to a message
    // must not exceed 6000 characters.
    interval_member!(EMBED_TOTAL_TEXT_LEN_INTERVAL, usize, 0, 6000);

    pub fn allow_mentions(
        &mut self,
        parse: Option<Vec<AllowedMention>>,
        roles: Option<Vec<Snowflake>>,
        users: Option<Vec<Snowflake>>,
        replied_user: bool,
    ) -> &mut Self {
        self.allow_mentions = Some(AllowedMentions::new(parse, roles, users, replied_user));
        self
    }
}

#[derive(Serialize, Debug)]
pub struct Embed {
    pub title: Option<String>,
    #[serde(rename = "type")]
    embed_type: String,
    pub description: Option<String>,
    pub url: Option<String>,
    // ISO8601,
    pub timestamp: Option<String>,
    pub color: Option<String>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<EmbedImage>,
    pub video: Option<EmbedVideo>,
    pub thumbnail: Option<EmbedThumbnail>,
    pub provider: Option<EmbedProvider>,
    pub author: Option<EmbedAuthor>,
    pub fields: Vec<EmbedField>,
}

impl Embed {
    pub fn new() -> Self {
        Self {
            title: None,
            embed_type: String::from("rich"),
            description: None,
            url: None,
            timestamp: None,
            color: None,
            footer: None,
            image: None,
            video: None,
            thumbnail: None,
            provider: None,
            author: None,
            fields: vec![],
        }
    }

    pub fn title(&mut self, title: &str) -> &mut Self {
        self.title = Some(title.to_owned());
        self
    }

    pub fn description(&mut self, description: &str) -> &mut Self {
        self.description = Some(description.to_owned());
        self
    }

    pub fn url(&mut self, url: &str) -> &mut Self {
        self.url = Some(url.to_owned());
        self
    }

    pub fn timestamp(&mut self, timestamp: &str) -> &mut Self {
        self.timestamp = Some(timestamp.to_owned());
        self
    }

    pub fn color(&mut self, color: &str) -> &mut Self {
        self.color = Some(color.to_owned());
        self
    }

    pub fn footer(&mut self, text: &str, icon_url: Option<String>) -> &mut Self {
        self.footer = Some(EmbedFooter::new(text, icon_url));
        self
    }

    pub fn image(&mut self, url: &str) -> &mut Self {
        self.image = Some(EmbedImage::new(url));
        self
    }

    pub fn video(&mut self, url: &str) -> &mut Self {
        self.video = Some(EmbedVideo::new(url));
        self
    }

    pub fn thumbnail(&mut self, url: &str) -> &mut Self {
        self.thumbnail = Some(EmbedThumbnail::new(url));
        self
    }

    pub fn provider(&mut self, name: &str, url: &str) -> &mut Self {
        self.provider = Some(EmbedProvider::new(name, url));
        self
    }

    pub fn author(
        &mut self,
        name: &str,
        url: Option<String>,
        icon_url: Option<String>,
    ) -> &mut Self {
        self.author = Some(EmbedAuthor::new(name, url, icon_url));
        self
    }

    pub fn field(&mut self, name: &str, value: &str, inline: bool) -> &mut Self {
        if self.fields.len() == Embed::FIELDS_LEN_INTERVAL.max_allowed {
            panic!("You can't have more than {} fields in an embed!", Embed::FIELDS_LEN_INTERVAL.max_allowed)
        }

        self.fields.push(EmbedField::new(name, value, inline));
        self
    }

    interval_member!(TITLE_LEN_INTERVAL, usize, 0, 256);
    interval_member!(DESCRIPTION_LEN_INTERVAL, usize, 0, 4096);
    // enforced in field... by panic though... todo!
    interval_member!(FIELDS_LEN_INTERVAL, usize, 0, 25);
}

#[derive(Serialize, Debug)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

impl EmbedField {
    pub fn new(name: &str, value: &str, inline: bool) -> Self {
        Self {
            name: name.to_owned(),
            value: value.to_owned(),
            inline,
        }
    }
    interval_member!(NAME_LEN_INTERVAL, usize, 0, 256);
    interval_member!(VALUE_LEN_INTERVAL, usize, 0, 1024);
}

#[derive(Serialize, Debug)]
pub struct EmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

impl EmbedFooter {
    pub fn new(text: &str, icon_url: Option<String>) -> Self {
        Self {
            text: text.to_owned(),
            icon_url,
        }
    }
    interval_member!(TEXT_LEN_INTERVAL, usize, 0, 2048);
}

pub type EmbedImage = EmbedUrlSource;
pub type EmbedThumbnail = EmbedUrlSource;
pub type EmbedVideo = EmbedUrlSource;

#[derive(Serialize, Debug)]
pub struct EmbedUrlSource {
    pub url: String,
}

impl EmbedUrlSource {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_owned(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct EmbedProvider {
    pub name: String,
    pub url: String,
}

impl EmbedProvider {
    pub fn new(name: &str, url: &str) -> Self {
        Self {
            name: name.to_owned(),
            url: url.to_owned(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct EmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
}

impl EmbedAuthor {
    pub fn new(name: &str, url: Option<String>, icon_url: Option<String>) -> Self {
        Self {
            name: name.to_owned(),
            url,
            icon_url,
        }
    }
    interval_member!(NAME_LEN_INTERVAL, usize, 0, 256);
}

pub enum AllowedMention {
    RoleMention,
    UserMention,
    EveryoneMention,
}

fn resolve_allowed_mention_name(allowed_mention: AllowedMention) -> String {
    match allowed_mention {
        AllowedMention::RoleMention => "roles".to_string(),
        AllowedMention::UserMention => "users".to_string(),
        AllowedMention::EveryoneMention => "everyone".to_string(),
    }
}

#[derive(Serialize, Debug)]
pub struct AllowedMentions {
    pub parse: Option<Vec<String>>,
    pub roles: Option<Vec<Snowflake>>,
    pub users: Option<Vec<Snowflake>>,
    pub replied_user: bool,
}

impl AllowedMentions {
    pub fn new(
        parse: Option<Vec<AllowedMention>>,
        roles: Option<Vec<Snowflake>>,
        users: Option<Vec<Snowflake>>,
        replied_user: bool,
    ) -> Self {
        let mut parse_strings: Vec<String> = vec![];
        if parse.is_some() {
            parse
                .unwrap()
                .into_iter()
                .for_each(|x| parse_strings.push(resolve_allowed_mention_name(x)))
        }

        Self {
            parse: Some(parse_strings),
            roles,
            users,
            replied_user,
        }
    }
}

// ready to be extended with other components
// non-composite here specifically means *not an action row*
#[derive(Debug)]
enum NonCompositeComponent {
    Button(Button),
}

impl Serialize for NonCompositeComponent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            NonCompositeComponent::Button(button) => button.serialize(serializer),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ActionRow {
    #[serde(rename = "type")]
    pub component_type: u8,
    components: Vec<NonCompositeComponent>,
}

impl ActionRow {
    fn new() -> ActionRow {
        ActionRow {
            component_type: 1,
            components: vec![],
        }
    }

    pub fn link_button<Func>(&mut self, button_mutator: Func) -> &mut Self
    where
        Func: Fn(&mut LinkButton) -> &mut LinkButton,
    {
        let mut button = LinkButton::new();
        button_mutator(&mut button);
        self.components.push(NonCompositeComponent::Button(
            button.to_serializable_button(),
        ));
        self
    }

    pub fn regular_button<Func>(&mut self, button_mutator: Func) -> &mut Self
    where
        Func: Fn(&mut RegularButton) -> &mut RegularButton,
    {
        let mut button = RegularButton::new();
        button_mutator(&mut button);
        self.components.push(NonCompositeComponent::Button(
            button.to_serializable_button(),
        ));
        self
    }
    interval_member!(BUTTON_COUNT_INTERVAL, usize, 0, 5);
}

#[derive(Debug, Clone)]
pub enum NonLinkButtonStyle {
    Primary,
    Secondary,
    Success,
    Danger,
}

impl NonLinkButtonStyle {
    fn get_button_style(&self) -> ButtonStyles {
        match *self {
            NonLinkButtonStyle::Primary => ButtonStyles::Primary,
            NonLinkButtonStyle::Secondary => ButtonStyles::Secondary,
            NonLinkButtonStyle::Success => ButtonStyles::Success,
            NonLinkButtonStyle::Danger => ButtonStyles::Danger,
        }
    }
}

// since link button has an explicit way of creation via the action row
// this enum is kept hidden from the user ans the NonLinkButtonStyle is created to avoid
// user confusion
#[derive(Debug)]
enum ButtonStyles {
    Primary,
    Secondary,
    Success,
    Danger,
    Link,
}

impl Serialize for ButtonStyles {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let to_serialize = match *self {
            ButtonStyles::Primary => 1,
            ButtonStyles::Secondary => 2,
            ButtonStyles::Success => 3,
            ButtonStyles::Danger => 4,
            ButtonStyles::Link => 5,
        };
        serializer.serialize_i32(to_serialize)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct PartialEmoji {
    pub id: Snowflake,
    pub name: String,
    pub animated: Option<bool>,
}

/// the button struct intended for serialized
#[derive(Serialize, Debug)]
struct Button {
    #[serde(rename = "type")]
    pub component_type: i8,
    pub style: Option<ButtonStyles>,
    pub label: Option<String>,
    pub emoji: Option<PartialEmoji>,
    pub custom_id: Option<String>,
    pub url: Option<String>,
    pub disabled: Option<bool>,
}

impl Button {
    fn new(
        style: Option<ButtonStyles>,
        label: Option<String>,
        emoji: Option<PartialEmoji>,
        url: Option<String>,
        custom_id: Option<String>,
        disabled: Option<bool>,
    ) -> Self {
        Self {
            component_type: 2,
            style,
            label,
            emoji,
            url,
            custom_id,
            disabled,
        }
    }
}

/// Data holder for shared fields of link and regular buttons
#[derive(Debug)]
struct ButtonCommonBase {
    pub label: Option<String>,
    pub emoji: Option<PartialEmoji>,
    pub disabled: Option<bool>,
}

impl ButtonCommonBase {
    fn new(label: Option<String>, emoji: Option<PartialEmoji>, disabled: Option<bool>) -> Self {
        ButtonCommonBase {
            label,
            emoji,
            disabled,
        }
    }
    fn label(&mut self, label: &str) -> &mut Self {
        self.label = Some(label.to_string());
        self
    }

    fn emoji(&mut self, emoji_id: Snowflake, name: &str, animated: bool) -> &mut Self {
        self.emoji = Some(PartialEmoji {
            id: emoji_id,
            name: name.to_string(),
            animated: Some(animated),
        });
        self
    }

    fn disabled(&mut self, disabled: bool) -> &mut Self {
        self.disabled = Some(disabled);
        self
    }
}

/// a macro which takes an identifier (`base`) of the ButtonCommonBase (relative to `self`)
/// and generates setter functions that delegate their inputs to the `self.base`
macro_rules! button_base_delegation {
    ($base:ident) => {
        pub fn emoji(&mut self, emoji_id: &str, name: &str, animated: bool) -> &mut Self {
            self.$base.emoji(emoji_id.to_string(), name, animated);
            self
        }

        pub fn disabled(&mut self, disabled: bool) -> &mut Self {
            self.$base.disabled(disabled);
            self
        }

        pub fn label(&mut self, label: &str) -> &mut Self {
            self.$base.label(label);
            self
        }
    };
}

#[derive(Debug)]
pub struct LinkButton {
    button_base: ButtonCommonBase,
    url: Option<String>,
}

impl LinkButton {
    fn new() -> Self {
        LinkButton {
            button_base: ButtonCommonBase::new(None, None, None),
            url: None,
        }
    }

    pub fn url(&mut self, url: &str) -> &mut Self {
        self.url = Some(url.to_string());
        self
    }

    button_base_delegation!(button_base);
}

pub struct RegularButton {
    button_base: ButtonCommonBase,
    custom_id: Option<String>,
    style: Option<NonLinkButtonStyle>,
}

impl RegularButton {
    fn new() -> Self {
        RegularButton {
            button_base: ButtonCommonBase::new(None, None, None),
            custom_id: None,
            style: None,
        }
    }

    pub fn custom_id(&mut self, custom_id: &str) -> &mut Self {
        self.custom_id = Some(custom_id.to_string());
        self
    }

    pub fn style(&mut self, style: NonLinkButtonStyle) -> &mut Self {
        self.style = Some(style);
        self
    }

    button_base_delegation!(button_base);
}

trait ToSerializableButton {
    fn to_serializable_button(&self) -> Button;
}

impl ToSerializableButton for LinkButton {
    fn to_serializable_button(&self) -> Button {
        Button::new(
            Some(ButtonStyles::Link),
            self.button_base.label.clone(),
            self.button_base.emoji.clone(),
            self.url.clone(),
            None,
            self.button_base.disabled,
        )
    }
}

impl ToSerializableButton for RegularButton {
    fn to_serializable_button(&self) -> Button {
        Button::new(
            self.style.clone().map(|s| s.get_button_style()),
            self.button_base.label.clone(),
            self.button_base.emoji.clone(),
            None,
            self.custom_id.clone(),
            self.button_base.disabled,
        )
    }
}

/// A trait for checking that an API message component is compatible with the official Discord API constraints
///
/// This trait should be implemented for any components for which the Discord API documentation states
/// limitations (maximum count, maximum length, uniqueness with respect to other components, restrictions
/// on children components, ...)
pub(crate) trait DiscordApiCompatible {
    fn check_compatibility(&self, context: &mut MessageContext) -> Result<(), String>;
}

impl DiscordApiCompatible for NonCompositeComponent {
    fn check_compatibility(&self, context: &mut MessageContext) -> Result<(), String> {
        match self {
            NonCompositeComponent::Button(b) => b.check_compatibility(context),
        }
    }
}

impl DiscordApiCompatible for Button {
    fn check_compatibility(&self, context: &mut MessageContext) -> Result<(), String> {
        if let Some(label) = &self.label {
            interval_check(&Message::LABEL_LEN_INTERVAL, &label.len(), "Label length")?;
        }

        return match self.style {
            None => Err("Button style must be set!".to_string()),
            Some(ButtonStyles::Link) => {
                if self.url.is_none() {
                    Err("Url of a Link button must be set!".to_string())
                } else {
                    Ok(())
                }
            }
            // list all remaining in case a style with different requirements is added
            Some(ButtonStyles::Danger)
            | Some(ButtonStyles::Primary)
            | Some(ButtonStyles::Success)
            | Some(ButtonStyles::Secondary) => {
                return if let Some(id) = self.custom_id.as_ref() {
                    context.register_button(id)
                } else {
                    Err("Custom ID of a NonLink button must be set!".to_string())
                };
            }
        };
    }
}

impl DiscordApiCompatible for ActionRow {
    fn check_compatibility(&self, context: &mut MessageContext) -> Result<(), String> {
        context.register_action_row();
        if self.components.is_empty() {
            return Err("Empty action row detected!".to_string());
        }

        self.components.iter().fold(Ok(()), |acc, component| {
            acc.and(component.check_compatibility(context))
        })
    }
}

impl DiscordApiCompatible for Message {
    fn check_compatibility(&self, context: &mut MessageContext) -> Result<(), String> {
        interval_check(
            &Message::ACTION_ROW_COUNT_INTERVAL,
            &self.action_rows.len(),
            "Action row count")?;

        self.embeds
            .iter()
            .fold(Ok(()), |acc, emb| acc.and(emb.check_compatibility(context)))?;

        self.action_rows
            .iter()
            .fold(Ok(()), |acc, row| acc.and(row.check_compatibility(context)))
    }
}

impl DiscordApiCompatible for Embed {
    fn check_compatibility(&self, context: &mut MessageContext) -> Result<(), String> {
        context.register_embed(self)?;
        interval_check(&Self::FIELDS_LEN_INTERVAL, &self.fields.len(), "Embed field count")?;

        if let Some(title) = self.title.as_ref() {
            interval_check(&Self::TITLE_LEN_INTERVAL, &title.len(), "Embed title length")?;
        }

        if let Some(description) = self.description.as_ref() {
            interval_check(&Self::DESCRIPTION_LEN_INTERVAL, &description.len(), "Embed description length")?;
        }

        self.author.as_ref().map_or_else(|| Ok(()), |a| a.check_compatibility(context))?;
        self.footer.as_ref().map_or_else(|| Ok(()), |f| f.check_compatibility(context))?;

        for field in self.fields.iter() {
            field.check_compatibility(context)?;
        }
        Ok(())
    }
}

impl DiscordApiCompatible for EmbedAuthor {
    fn check_compatibility(&self, _context: &mut MessageContext) -> Result<(), String> {
        interval_check(&Self::NAME_LEN_INTERVAL, &self.name.len(), "Embed author name length")?;
        Ok(())
    }
}

impl DiscordApiCompatible for EmbedFooter {
    fn check_compatibility(&self, _context: &mut MessageContext) -> Result<(), String> {
        interval_check(&Self::TEXT_LEN_INTERVAL, &self.text.len(), "Embed footer text length")?;
        Ok(())
    }
}

impl DiscordApiCompatible for EmbedField {
    fn check_compatibility(&self, _context: &mut MessageContext) -> Result<(), String> {
        interval_check(&Self::VALUE_LEN_INTERVAL, &self.value.len(), "Embed field value length")?;
        interval_check(&Self::NAME_LEN_INTERVAL, &self.name.len(), "Embed field name length")?;
        Ok(())
    }
}
