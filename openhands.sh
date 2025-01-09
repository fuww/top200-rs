#!/bin/bash
docker pull docker.all-hands.dev/all-hands-ai/runtime:0.19-nikolaik

export WORKSPACE_BASE=$HOME/git/top200-rs

# export LLM_PROVIDER=OPENAI
export LLM_MODEL="anthropic/claude-3-5-sonnet-20241022"
# export OPENAI_API_KEY=sk-

export $(cat .env | xargs)

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

# Vertex ai

    # -e GOOGLE_APPLICATION_CREDENTIALS="<json-dump-of-gcp-service-account-json>" \
    # -e VERTEXAI_PROJECT="<your-gcp-project-id>" \
    # -e VERTEXAI_LOCATION="<your-gcp-location>" \