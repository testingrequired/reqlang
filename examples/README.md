# Examples

This directory contains example request files. Here are some highlights.

## Get HTML from example.com

See: [get_example_com.reqlang](./get_example_com.reqlang)

```bash
$ reqlang-export ./examples/get_example_com.reqlang --format=curl | bash
```

## Get all the subreddits on the reddit homepage

See: [get_reddit_com_json.reqlang](./get_reddit_com_json.reqlang)

```bash
$ reqlang-export ./examples/get_reddit_com_json.reqlang --format=curl | bash | jq '.["data"]["children"][]["data"]["subreddit"]
```
