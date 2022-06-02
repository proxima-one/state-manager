# State Manager
A service which allows streaming applications to store their state, partially
update it and rollback to previous versions.

[Confluence page](https://proxima-one.atlassian.net/wiki/spaces/DEV/pages/220430337/State+Manager+API)

## Usage
Basically, a state manager for the given app is a key-value storage of byte
arrays. Current values can be fetched and new values can be set (either one by
one or in batches). There is also support for rollbacks. After storing some
values, the user can create a checkpoint. Later, the user can rollback to any
existing checkpoint. All the key-value pairs will then be restored to the point
in time when the checkpoint was created. All the following checkpoints will be
dropped.
The user can also request old checkpoints to be removed to free some space.

## Interface
Currently, the service only supports gRPC interface. Schema can be found
[here](https://github.com/proxima-one/state-manager/blob/master/proto/state_manager/state_manager.proto)
