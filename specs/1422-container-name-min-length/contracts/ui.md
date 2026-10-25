# Contract: container daemon form (additions to 1116)

- Save with a resolved container name (typed trimmed, else default) of 1 character: refused, "A container name needs at least 2 characters." on the container name field; no daemon created or changed.
- Empty container name and empty or unusable daemon name: still "Enter a container name." (or "A daemon needs a name." when the daemon name is empty).
- Placeholder: shown only when the default is 2 or more characters; hidden otherwise.
- Edit of a stored one-character daemon: loads and lists unchanged; Save with the name unchanged is refused with the same message.
- Too-short name never shows the duplicate message. Other runtimes, layout and fields unchanged.
