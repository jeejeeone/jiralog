# Jiralog

Update Jira worklog from command line with helpful utilities, soothe the jira pain... 

Works with Jira api v3.

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

## Begin

```
Begin work item, ends previous work, records time automatically

Usage: jiralog begin [OPTIONS] <TICKET>

Arguments:
  <TICKET>

Options:
  -d, --description <DESCRIPTION>  Add description for work
```

Example:

```
> jiralog begin ABC-1
Begin a903aabda8: ticket=ABC-1

> jiralog begin ABC-2
End b839e77b57: time spent=1m
Begin 0ddc328876: ticket=ABC-2
```

## Add

```
Add work item, by default started date is current time

Usage: jiralog add [OPTIONS] <TICKET> <TIME_SPENT>

Arguments:
  <TICKET>
  <TIME_SPENT>  Time spent in Jira format, for example 1d5h

Options:
  -s, --started-date <STARTED_DATE>  Provide start date for work item in format 'YYYY-MM-DDTHH:MM' or 'HH:MM'. HH:MM defaults to current day
  -d, --description <DESCRIPTION>    Add description for work
```

```
jiralog add ABC-2 1d3h
Added aa08dbe84b: ticket=ABC-2, time spent=1d3h, started_date=2024-12-07 16:00:53.461856 +02:00, description=''
```
