# T1 Bot

The T1 Bot is an extensible and easy to setup Matrix room moderation bot. It
provides various moderation features to help manage and secure your Matrix
rooms.

## Features

- **Quiz Captcha**: Challenge new users with a quiz to verify they are human.
- **Link Spam Detection**: Monitor and control the posting of links to prevent spam.
- **Rate Limiting**: Limit the rate of messages to prevent flooding.

## Configuration

The bot is configured using a `config.toml` file. Below is an example configuration:

To enable the bot with basic configuration first you need to create an account
for the bot, then add it to the room and give it moderation permission.

```toml
[t1bot]
user_id = "@t1:example.org"
password = "Bot login password"
display_name = "Robo T1"
device_id = "random uuid"
device_name = "any device name"

[state_store]
path = "/path/to/state/store"
password = "optional_password"

# Rate limiting uses token bucket algorithm, each new token allows one messsage
[monitors.rate_limit]
token_new = 3
token_new_max = 3
token_new_timeout_secs = 40
token_join = 10
token_join_max = 30
fill_rate = 10
fill_freq_secs = 10

[monitors.link_spam]
watch_timeout_secs = 40

# Room ID can be found from room tech details
[rooms."!SkUFfRbJYMZsbBMRcWylf:example.org"]
enabled = true
# Room specific settings
monitors.captcha.timeout_secs = 60

# Questions can be customized for each room.
# Upon new user join, one question will be randomly picked from the question set.
# Currently supports maximum 5 answers.
[[rooms."!SkUFfRbJYMZsbBMRcWylf:example.org".monitors.captcha.questions]]
body = "Answer this question or get kicked, are you a robot? 1. Yes 2. No"
answer = 2
```

For more detailed configuration options, refer to the `config.rs` file.

## License

This project is licensed under either the MIT License or the Apache License 2.0,
at your option.

## Contributions

By contributing to this project, you agree that your contributions will be
licensed under the same licenses as specified in the [License](#license)
section.