use serde::{de::DeserializeOwned, Serialize};
use serenity::{
    model::{
        channel::{
            Channel, ChannelType, GuildChannel, PermissionOverwrite, PermissionOverwriteType,
        },
        guild::Guild,
        id::RoleId,
        Permissions,
    },
    prelude::*,
};
use std::{
    collections::BTreeMap,
    error::Error as StdError,
    fmt::{Debug, Display},
};

#[derive(Debug)]
pub enum Error {
    SerenityError(serenity::Error),
    RonError(ron::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SerenityError(err) => write!(f, "SerenityError: {}", err),
            Error::RonError(err) => write!(f, "RonError: {}", err),
        }
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::SerenityError(err) => Some(err),
            Self::RonError(err) => Some(err),
        }
    }
}
impl From<serenity::Error> for Error {
    fn from(err: serenity::Error) -> Self {
        Self::SerenityError(err)
    }
}
impl From<ron::Error> for Error {
    fn from(err: ron::Error) -> Self {
        Self::RonError(err)
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct Storage<K, V>
where
    K: Ord + Debug + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + Debug + Send + Sync,
{
    pub guild: Guild,
    pub data: BTreeMap<K, V>,
    pub channel: GuildChannel,
    pub ctx: Box<Context>,
}

impl<K, V> Debug for Storage<K, V>
where
    K: Ord + Debug + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + Debug + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Storage {{ guild: {:?}, data: {:?}, channel: {:?} }}",
            self.guild, self.data, self.channel
        )
    }
}

impl<K, V> Storage<K, V>
where
    K: Ord + Debug + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + Debug + Send + Sync,
{
    pub async fn new(mut guild: Guild, ctx: Box<Context>) -> Result<Self> {
        let channel_name = "storage-for-a-bot".to_owned();
        let channel = guild.channels.values_mut().find(|c| {
            if let Channel::Guild(c) = c {
                c.name == channel_name
            } else {
                false
            }
        });

        let channel = match channel {
            Some(c) => match c {
                Channel::Guild(c) => c.clone(),
                _ => unreachable!(),
            },
            None => {
                guild
                    .create_channel(&ctx.http, |c| {
                        c.name(channel_name)
                            .kind(ChannelType::Text)
                            .position(0)
                            .permissions([PermissionOverwrite {
                                allow: Permissions::empty(),
                                deny: Permissions::READ_MESSAGES,
                                // Applies to @everyone
                                kind: PermissionOverwriteType::Role(RoleId::from(
                                    *guild.id.as_u64(),
                                )),
                            }])
                    })
                    .await?
            }
        };

        let mut self_ = Self {
            guild,
            data: BTreeMap::new(),
            channel,
            ctx,
        };

        self_.get_latest_from_channel().await?;

        Ok(self_)
    }

    pub async fn get_latest_from_channel(&mut self) -> Result<()> {
        let latest = self
            .channel
            .messages(&self.ctx.http, |m| m.limit(100))
            .await?
            .into_iter()
            .map(|message| message.content)
            .collect::<String>();

        self.data = ron::from_str(&latest).unwrap_or_else(|_| BTreeMap::new());

        Ok(())
    }

    pub async fn write(&mut self, key: K, value: V) -> Result<()> {
        use serenity::{http::error, Error};
        self.data.insert(key, value);

        let messages = self
            .channel
            .messages(&self.ctx.http, |m| m.limit(100))
            .await
            .and_then(|e| {
                if let Error::Http(e) = e {
                    if let error::Error(error::ErrorResponse(error::DiscordJsonError { code, .. }))
                } else {
                    Err(e)
                }
            });

        for message in self
            .channel
            .messages(&self.ctx.http, |m| m.limit(100))
            .await
            .map_err(|e| match e {
                _ => e,
            })?
        {
            message.delete((&self.ctx.cache, &*self.ctx.http)).await?;
        }

        for chunk in ron::to_string(&self.data)?
            .as_bytes()
            .chunks(2000)
            .map(String::from_utf8_lossy)
        {
            self.channel
                .send_message((&self.ctx.cache, &*self.ctx.http), |m| m.content(chunk))
                .await?;
        }

        Ok(())
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.data.get_mut(key)
    }
}
