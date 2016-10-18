def main(request, response):
    path = request.GET["path"] if "path" in request.GET else None
    print("%s ==> %s || %s" % (request.method, request.GET["id"], path))
    if request.method == 'POST':
        request.server.stash.put(request.GET["id"], request.body, path)
        return ''

    x = request.server.stash.take(request.GET["id"], path)
    print(x)
    return x
