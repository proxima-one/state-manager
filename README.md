# State Manager
A service that allows streaming applications to store their state, partially
update it and rollback to previous versions.

## Usage
Basically, a state manager for the given app is a key-value storage of byte
arrays. Current values can be fetched and new values can be set (either one by
one or in batches). There is also support for rollbacks. After storing some
values, the user can create a checkpoint. Later, the user can rollback to any
existing checkpoint. All the key-value pairs will then be restored to the point
in time when the checkpoint was created. All the following checkpoints will be
dropped.\
Old checkpoints are automatically cleaned up with more fresh checkpoints being left than stale ones.

## Interface
Currently, the service only supports gRPC interface. Schema can be found
[here](/proto/state_manager/state_manager.proto)

## About etags
An `etag` is some string used to avoid concurrent modifications. The client receives an etag with every response and should provide it with the next request to the server. The server then checks that no modifications have happened since the moment of response with the corresponding etag. Read-only requests don’t require an etag.
Etags are independent across applications. Modifying requests to one application don’t affect the behavior of requests to another application.
