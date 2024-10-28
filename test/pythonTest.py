import boto3
import json

# Initialize Bedrock client with credentials
def create_bedrock_client():
    return boto3.client(
        'bedrock-runtime',
        region_name='us-east-1',  # Replace with your region
        aws_access_key_id='AKIAZ3MGMXZCXS4J2IQN',  # Replace with your Access Key
        aws_secret_access_key='+/Uahbufvc2Y3A+/+dmy7JiHxvcgwGpx7zU9DX5B'  # Replace with your Secret Key
    )

# Function to send question to the Bedrock API
def ask_bedrock(client, question):
    response = client.invoke_model(
        modelId='amazon.titan-text-express-v1',  # Replace with your Bedrock model ID
        body=json.dumps({
            "inputText": question
        }),
        contentType='application/json'
    )
    response_body = response['body'].read().decode('utf-8')

    result = json.loads(response_body)

    print("Full Response:", result)

if __name__ == "__main__":
    # Get question from user
    question = input("Enter your question: ")

    # Create the Bedrock client with credentials
    bedrock_client = create_bedrock_client()

    # Send the question and get a response
    ask_bedrock(bedrock_client, question)
