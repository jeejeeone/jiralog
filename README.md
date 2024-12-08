# Jiralog

Update Jira issue worklog from command line with helpful utilities, soothe the jira pain... Works with Jira api v3.

<p align="left"><img src="/demo/crop-demo.gif?raw=true"/></p>

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
  show       Show worklog in explorer tui, optionally to stdout
  configure  Configure jiralog
  info       Print info
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Workflow

**Record time while you go**

> Note! commit ends current work
```
jj@jj worklog % jiralog begin ABC-1
Begin d32e8c4df9: ticket=ABC-1

jj@jj worklog % jiralog begin ABC-2 # Ends work for ABC-1, add duration automatically
End d32e8c4df9: ticket=ABC-1, time spent=5m

Begin e3a238906f: ticket=ABC-2
jj@jj ~ % jiralog end
End e3a238906f: ticket=ABC-2, time spent=1m # Stop current work, add duration. Note! Commit also ends current work
```

**Add worklog items**
```
# Start date from now
jj@jj worklog % jiralog add ABC-2 3h
Added a9c99c703a: ticket=ABC-2, time spent=3h, started_date=2024-12-07 21:48:07.002467 +02:00, description=  

# Start date within current day
jj@jj worklog % jiralog add ABC-2 3h --start-date 9:30  
Added d37b129482: ticket=ABC-2, time spent=3h, started_date=2024-12-07 09:30:00 +02:00, description=

# Start date from datetime
jj@jj worklog % jiralog add ABC-5 3h --start-date 2024-09-09T10:10  
Added 00719956af: ticket=ABC-5, time spent=3h, started_date=2024-09-09 10:10:00 +02:00, description=
```

**Commit worklog to Jira**
```
# Open editor to edit entries before commit, removing all entries aborts commit
jj@jj worklog % jiralog commit
```

**Remove worklog items**
```
# Remove previous entry
jj@jj worklog % jiralog pop
Removed 00719956af: ticket=ABC-5, time spent=3h, description=

# Remove some entry
jj@jj worklog % jiralog rm a9c99c703a
Removed a9c99c703a

# Remove committed items
jj@jj worklog % jiralog purge
Removed 7 items
```

**Explore worklog items**

```
# Explore items with explorer tui
jj@jj worklog % jiralog show

# Items to stdout
jj@jj worklog % jiralog show --stdout
ticket,time_spent,description,started_date,committed,id
ABC-1,1m,,2024-12-07T21:11:44.827321+02:00,true,1467c62b9c
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
