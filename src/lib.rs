use serenity::{
    model::{
        channel::{Channel, ChannelType, GuildChannel},
        guild::Guild,
    },
    prelude::*,
    Result as SResult,
};
use std::{any::Any, collections::BTreeMap, fmt::Debug};

pub struct Storage<T: Ord + Debug> {
    pub guild: Guild,
    pub data: BTreeMap<T, Box<dyn Any + Send + Sync>>,
    pub channel: GuildChannel,
    pub ctx: Box<Context>,
}

impl<T: Ord + Debug> Debug for Storage<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Storage {{ guild: {:?}, data: {:?}, channel: {:?} }}",
            self.guild, self.data, self.channel
        )
    }
}

pub enum GetError {
    NotFound,
    WrongType,
}

impl<T: Ord + Debug> Storage<T> {
    pub async fn new(mut guild: Guild, ctx: Box<Context>) -> SResult<Self> {
        let channel = guild.channels.values_mut().find(|c| {
            if let Channel::Guild(c) = c {
                c.name == "storage"
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
                        c.name("storage")
                            .kind(ChannelType::Text)
                            .position(i32::MAX as u32)
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

    pub async fn get_latest_from_channel(&mut self) -> SResult<()> {
        let messages = self
            .channel
            .messages(&self.ctx.http, |m| m.limit(100))
            .await?;

        let mut out = String::new();

        for message in messages {
            out.push_str(&message.content);
        }

        println!("{}", out);

        Ok(())
    }

    pub fn write(&mut self, key: T, value: Box<dyn Any + Send + Sync>) {
        self.data.insert(key, value);
    }

    pub fn get<U: 'static>(&self, key: &T) -> Result<&U, GetError> {
        self.data.get(key).map_or(Err(GetError::NotFound), |v| {
            v.downcast_ref::<U>().ok_or(GetError::WrongType)
        })
    }

    pub fn get_mut<U: 'static>(&mut self, key: &T) -> Result<&mut U, GetError> {
        self.data.get_mut(key).map_or(Err(GetError::NotFound), |v| {
            v.downcast_mut::<U>().ok_or(GetError::WrongType)
        })
    }
}
