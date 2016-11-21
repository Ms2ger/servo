import sys
import urlparse

def main(request, response):
    print("HULLO")
    headers = [
        ["Access-Control-Allow-Origin", "http://www2.web-platform.test:8001"],
        ["Access-Control-Allow-Credentials", "true"],
    ]
    cookie = request.cookies.get("test")
    if cookie is not None:
        body = "%s" % cookie
    else:
        body = "no cookie"
    response.set_cookie("test", "true")
    return headers, body
