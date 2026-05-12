from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()


class LoginRequest(BaseModel):
    email: str


@app.post("/login")
def login(payload: LoginRequest):
    return {"email": payload.email}
