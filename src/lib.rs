use serenity::{
    model::{
        channel::{Channel, ChannelType, GuildChannel},
        guild::Guild,
    },
    prelude::*,
    Result as SResult,
};
use std::{any::Any, collections::BTreeMap};

pub struct Storage<'a, T: Ord> {
    pub guild: Guild,
    pub data: BTreeMap<T, Box<dyn Any>>,
    pub channel: GuildChannel,
    pub ctx: &'a Context,
}

pub enum GetError {
    NotFound,
    WrongType,
}

impl<'a, T: Ord> Storage<'a, T> {
    pub async fn new(mut guild: Guild, ctx: &'a Context) -> SResult<Storage<'a, T>> {
        let channel = guild.channels.values_mut().find(|c| match c {
            Channel::Guild(c) => c.name == "storage",
            _ => unreachable!(),
        });

        let channel = match channel {
            Some(c) => match c {
                Channel::Guild(c) => c.clone(),
                _ => unreachable!(),
            },
            None => {
                guild
                    .create_channel(&ctx.http, |c| {
                        c.name("storage").kind(ChannelType::Text).position(u32::MAX)
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

    pub fn write(&mut self, key: T, value: Box<dyn Any>) {
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
