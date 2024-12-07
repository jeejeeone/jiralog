# Jiralog

Update Jira worklog from command line with helpful utilities, soothe the jira pain... Works with Jira api v3.

![jiralog demo](https://github.com/jeejeeone/jiralog/blob/main/demo/demo.gif)


```
Command line tool to update issue worklog in Jira

Usage: jiralog [COMMAND]

Commands:
  add        Add work item
  rm         Remove work item
  pop        Remove latest work item
  begin      Begin work item, ends previous work, records time automatically
  end        End current work
  current    Print current work item
  commit     Commit worklog to Jira
  purge      Remove committed entries from worklog
  show       Show worklog using csvlens, optionally to stdout
  configure  Configure jiralog
  info       Print info
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

# Configure

Requires Jira api token, username and cloud instance/url to run.

| Property  | Info |
| ------------- | ------------- |
| user  | Jira username  |
| token  | Jira api token  |
| jira_cloud_instance  | Jira cloud instance id  |
| jira_url  | Optionally provide url to jira, cloud instance wins if both defined  |
| editor  | Editor to open worklog on edit, respects `EDITOR` env variable, as a last resort default to `nano`|

## Automatic configuration

Run `jiralog configure` for setup.

## Properties file configuration

Create file `$home/.jiralog/jiralog.properties` with properties.

Example:
```
  token=my-token
  jira_cloud_instance=my-instance
  user=jj
  editor=nano
```


# Commands

TODO
