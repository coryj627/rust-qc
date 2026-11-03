def greeting(name):
    name = name.strip()
    return f"Hello, {name or 'stranger'}!"
