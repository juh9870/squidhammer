#!/usr/bin/env bash 

# nix run github:cargo2nix/cargo2nix -- -o

read -p "run clion? (Y/n/q): " response

case $response in
  [yY][eE][sS]|[yY]|"")
    # If user types Y/y, or presses Enter, run clion
    nix develop --command clion . & disown
    ;;
  [nN][oO]|[nN])
    # If user types N/n, just run nix develop
    nix develop
    ;;
  [qQ])
    # If user types Q/q, quit the script
    echo "Quitting..."
    exit 0
    ;;
  *)
    echo "Invalid response. Please run the script again and respond with Y, N, or Q."
    exit 1
    ;;
esac