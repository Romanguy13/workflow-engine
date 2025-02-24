#!/bin/bash

# Load environment variables from .env or .env.local
if [ -f .env ]; then
  export $(cat .env | xargs)
elif [ -f .env.local ]; then
  export $(cat .env.local | xargs)
else
  echo "No environment file found. Please create a .env or .env.local file."
  exit 1
fi

# Build the project
cargo build

# Check if the database file exists
if [ ! -f "workflows.db" ]; then
  echo "Creating the database..."
  sqlx db create
else
  echo "Database already exists."
fi