from test import Container

container = Container(name="pod", image="image")
print(container.SerializeToString())
