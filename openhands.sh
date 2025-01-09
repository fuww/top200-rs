docker pull docker.all-hands.dev/all-hands-ai/runtime:0.19-nikolaik

export WORKSPACE_BASE=$HOME/git/top200-rs

# export LLM_PROVIDER=OPENAI
# export LLM_MODEL=gpt-4o
# export OPENAI_API_KEY=sk-

docker run -it --rm --pull=always \
    -e SANDBOX_USER_ID=$(id -u) \
    -e WORKSPACE_MOUNT_PATH=$WORKSPACE_BASE \
    -v $WORKSPACE_BASE:/opt/workspace_base \
    -e SANDBOX_RUNTIME_CONTAINER_IMAGE=docker.all-hands.dev/all-hands-ai/runtime:0.19-nikolaik \
    -e LOG_ALL_EVENTS=true \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v ~/.openhands-state:/.openhands-state \
    -p 3000:3000 \
    --add-host host.docker.internal:host-gateway \
    --name openhands-app \
    docker.all-hands.dev/all-hands-ai/openhands:0.19

    